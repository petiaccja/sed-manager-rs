use core::task::Poll;
use core::time::Duration;
use std::sync::Arc;

use tokio::sync::{mpsc, oneshot};

use crate::device::Device;
use crate::messaging::com_id::{HandleComIdRequest, HandleComIdResponse};
use crate::messaging::discovery::Discovery;
use crate::messaging::packet::ComPacket;
use crate::rpc::{Error, PackagedMethod, Properties, SessionIdentifier};
use crate::serialization::DeserializeBinary;

use super::command::Command;
use super::promise::Promise;
use super::receive_packet::{self, commit, ReceivePacket};
use super::send_packet::{self, SendPacket};
use super::shared::buffer::Buffer;
use super::shared::pipe::{SinkPipe, SourcePipe};
use super::sync_protocol::{roundtrip_com_id, roundtrip_packet};
use super::CommandSender;

pub struct Protocol {
    rx: mpsc::UnboundedReceiver<Command>,
    device: Arc<dyn Device>,
    com_id: u16,
    properties: Properties,
    // Packet: send
    send_packet: SendPacket,
    send_input: Buffer<send_packet::Input>,
    send_output: Buffer<send_packet::Output>,
    send_done: Buffer<SessionIdentifier>,
    // Packet: receive
    receive_packet: ReceivePacket,
    recv_sender: Buffer<(SessionIdentifier, receive_packet::Sender)>,
    recv_com_packet: Buffer<ComPacket>,
    recv_done: Buffer<SessionIdentifier>,
    // ComID
    com_id_input: Buffer<Promise<HandleComIdRequest, HandleComIdResponse, Error>>,
    com_id_sender: Buffer<oneshot::Sender<Result<HandleComIdResponse, Error>>>,
    com_id_response: Buffer<Result<HandleComIdResponse, Error>>,
}

enum CommandBatch {
    OpenSession { id: SessionIdentifier, properties: Properties },
    CloseSession { id: SessionIdentifier },
    CloseComSession,
    TryShutdown,
    Method { buffer: Vec<(SessionIdentifier, Promise<PackagedMethod, PackagedMethod, Error>)> },
    ComId { buffer: Vec<Promise<HandleComIdRequest, HandleComIdResponse, Error>> },
    Discover { request: Promise<(), Discovery, Error> },
}

impl Protocol {
    pub fn spawn(
        device: Arc<dyn Device>,
        com_id: u16,
        com_id_ext: u16,
        properties: Properties,
    ) -> (CommandSender, oneshot::Receiver<()>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let (done_tx, done_rx) = oneshot::channel();
        let protocol = Self::new(rx, device, com_id, com_id_ext, properties);
        let _ = super::runtime::RUNTIME.spawn(protocol.run(done_tx));
        (CommandSender::new(tx), done_rx)
    }

    pub fn new(
        rx: mpsc::UnboundedReceiver<Command>,
        device: Arc<dyn Device>,
        com_id: u16,
        com_id_ext: u16,
        properties: Properties,
    ) -> Self {
        Self {
            rx,
            device,
            com_id,
            properties,
            send_packet: SendPacket::new(com_id, com_id_ext),
            send_input: Buffer::new(),
            send_output: Buffer::new(),
            send_done: Buffer::new(),
            receive_packet: ReceivePacket::new(),
            recv_sender: Buffer::new(),
            recv_com_packet: Buffer::new(),
            recv_done: Buffer::new(),
            com_id_input: Buffer::new(),
            com_id_sender: Buffer::new(),
            com_id_response: Buffer::new(),
        }
    }

    pub fn capabilities() -> Properties {
        use crate::messaging::packet::{COM_PACKET_HEADER_LEN, PACKET_HEADER_LEN, SUB_PACKET_HEADER_LEN};
        let max_transfer_len = 1048576;
        Properties {
            max_methods: usize::MAX,
            max_subpackets: usize::MAX,
            max_gross_packet_size: max_transfer_len - COM_PACKET_HEADER_LEN,
            max_packets: usize::MAX,
            max_gross_compacket_size: max_transfer_len,
            max_gross_compacket_response_size: max_transfer_len,
            max_ind_token_size: max_transfer_len - COM_PACKET_HEADER_LEN - PACKET_HEADER_LEN - SUB_PACKET_HEADER_LEN,
            max_agg_token_size: max_transfer_len - COM_PACKET_HEADER_LEN - PACKET_HEADER_LEN - SUB_PACKET_HEADER_LEN,
            continued_tokens: false,
            seq_numbers: false,
            ack_nak: false,
            asynchronous: true,
            buffer_mgmt: false,
            max_retries: 3,
            trans_timeout: Duration::from_secs(10),
            def_trans_timeout: Duration::from_secs(10),
        }
    }

    async fn run(mut self, done: oneshot::Sender<()>) {
        use tracing::Instrument as _;
        let span = tracing::span!(tracing::Level::DEBUG, "RPC protocol", com_id = self.com_id);
        async move {
            tracing::event!(tracing::Level::DEBUG, "Initialized");
            loop {
                let command_batches = self.recv_batches().await;
                for batch in command_batches {
                    self.enqueue_command_batch(batch);
                    self.update_send_packet();
                }
                self.update_send_packet(); // In case the above loop was empty. Only slight performance waste.

                self.roundtrip_all_com_id().await;
                self.roundtrip_all_packet().await;

                if self.com_id_response.is_done()
                    && self.com_id_sender.is_done()
                    && self.send_done.is_done()
                    && self.recv_done.is_done()
                {
                    break;
                }
            }
            let _ = done.send(());
            tracing::event!(tracing::Level::DEBUG, "Shut down");
        }
        .instrument(span)
        .await;
    }

    async fn recv_batches(&mut self) -> Vec<CommandBatch> {
        let mut batches = Vec::<CommandBatch>::new();
        if let Ok(Some(command)) = tokio::time::timeout(Duration::from_millis(64), self.rx.recv()).await {
            append_command(&mut batches, command);
        }
        while let Ok(command) = self.rx.try_recv() {
            append_command(&mut batches, command);
        }
        batches
    }

    fn enqueue_command_batch(&mut self, command_batch: CommandBatch) {
        match command_batch {
            CommandBatch::OpenSession { id, properties } => {
                self.send_packet.open_session(id, properties.clone());
                self.receive_packet.open_session(id, properties);
            }
            CommandBatch::CloseSession { id } => {
                // We do not close the receive half. That will be closed once
                // the send half indicates the session is "done".
                self.send_packet.close_session(id);
            }
            CommandBatch::CloseComSession => {
                self.com_id_input.close();
            }
            CommandBatch::TryShutdown => {
                if self.send_packet.is_empty() && self.receive_packet.is_empty() {
                    self.send_done.close();
                    self.recv_done.close();
                }
            }
            CommandBatch::Method { buffer } => {
                for request in buffer {
                    self.send_input.push(request);
                }
            }
            CommandBatch::ComId { buffer } => {
                for request in buffer {
                    self.com_id_input.push(request);
                }
            }
            CommandBatch::Discover { request } => {
                let (_, mut promises) = request.detach();
                promises.pop().map(|pr| drop(pr.send(discover(&*self.device))));
            }
        }
    }

    fn update_send_packet(&mut self) {
        self.send_packet.update(&mut self.send_input, &mut self.send_output, &mut self.send_done);
    }

    fn update_receive_packet(&mut self) {
        self.receive_packet.update(&mut self.recv_sender, &mut self.recv_com_packet, &mut self.recv_done);
    }

    fn update_com_id(&mut self) {
        commit(&mut self.com_id_sender, &mut self.com_id_response);
        if self.com_id_input.is_done() {
            self.com_id_sender.close();
            self.com_id_response.close();
        }
    }

    fn finalize_sessions(&mut self) {
        // Receive sessions should only be closed once the associated send session
        // has exhausted all its inputs.
        while let Poll::Ready(Some(id)) = self.send_done.pop() {
            self.receive_packet.close_session(id);
        }
        // In case a recv session aborts, abort the associated send session.
        // This code attempts to abort normally closed sessions as well, but
        // that's not an issue as by that time the send session is closed anyway.
        while let Poll::Ready(Some(id)) = self.recv_done.pop() {
            self.send_packet.abort_session(id);
        }
    }

    async fn roundtrip_all_packet(&mut self) {
        while let Poll::Ready(Some((com_packet, promises))) = self.send_output.pop() {
            let response = roundtrip_packet(&*self.device, self.com_id, com_packet, &self.properties).await;
            match response {
                Ok(com_packet) => {
                    self.recv_com_packet.push(com_packet);
                    for pr in promises {
                        let (id, senders) = pr.detach();
                        for sender in senders {
                            self.recv_sender.push((id, sender));
                        }
                    }
                }
                Err(error) => {
                    for pr in promises {
                        pr.close_with_error(error.clone());
                    }
                }
            }
            self.update_receive_packet();
            self.finalize_sessions();
        }
        // Handle any residual stuff.
        self.update_receive_packet();
        self.finalize_sessions();
    }

    async fn roundtrip_all_com_id(&mut self) {
        while let Poll::Ready(Some(pr)) = self.com_id_input.pop() {
            let (request, senders) = pr.detach();
            let response = roundtrip_com_id(&*self.device, self.com_id, request, &self.properties).await;
            self.com_id_response.push(response);
            for sender in senders {
                self.com_id_sender.push(sender);
            }
            self.update_com_id();
        }
        // Handle any residual stuff.
        self.update_com_id();
    }
}

fn append_command(batches: &mut Vec<CommandBatch>, command: Command) {
    // The order of the matchers matters!
    match (batches.last_mut(), command) {
        (Some(CommandBatch::Method { buffer }), Command::Method { id, request }) => buffer.push((id, request)),
        (Some(CommandBatch::ComId { buffer }), Command::ComId { request }) => buffer.push(request),
        (_, Command::Method { id, request }) => batches.push(CommandBatch::Method { buffer: vec![(id, request)] }),
        (_, Command::ComId { request }) => batches.push(CommandBatch::ComId { buffer: vec![request] }),
        (_, Command::OpenSession { id, properties }) => batches.push(CommandBatch::OpenSession { id, properties }),
        (_, Command::CloseSession { id }) => batches.push(CommandBatch::CloseSession { id }),
        (_, Command::Discover { request }) => batches.push(CommandBatch::Discover { request }),
        (_, Command::CloseComSession) => batches.push(CommandBatch::CloseComSession),
        (_, Command::TryShutdown) => batches.push(CommandBatch::TryShutdown),
    }
}

pub fn discover(device: &dyn Device) -> Result<Discovery, Error> {
    let data = device.security_recv(0x01, 0x0001_u16.to_be_bytes(), 4096)?;
    let discovery = Discovery::from_bytes(data)?;
    Ok(discovery.remove_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        fake_device::{FakeDevice, BASE_COM_ID},
        rpc::MethodCall,
        spec::{invoking_id::SESSION_MANAGER, sm_method_id::PROPERTIES},
    };

    #[tokio::test]
    async fn send_com_id_success() {
        let device = Arc::new(FakeDevice::new()) as Arc<dyn Device>;
        let (command, done) = Protocol::spawn(device, BASE_COM_ID, 0, Properties::ASSUMED);

        let result = command.com_id(HandleComIdRequest::verify_com_id_valid(BASE_COM_ID, 0)).await;
        assert!(result.is_ok());

        command.close_com_session();
        command.try_shutdown();
        drop(command);
        let _ = done.await;
    }

    #[tokio::test]
    async fn send_session_success() {
        let device = Arc::new(FakeDevice::new()) as Arc<dyn Device>;
        let (command, done) = Protocol::spawn(device, BASE_COM_ID, 0, Properties::ASSUMED);
        let id = SessionIdentifier { hsn: 0, tsn: 0 };

        command.open_session(id, Properties::ASSUMED);
        let request = PackagedMethod::Call(MethodCall::new_success(SESSION_MANAGER, PROPERTIES, vec![]));
        let result = command.method(id, request).await;
        assert!(result.is_ok());
        command.close_session(id);

        command.close_com_session();
        command.try_shutdown();
        drop(command);
        let _ = done.await;
    }
}
