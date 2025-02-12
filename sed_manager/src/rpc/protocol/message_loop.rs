use std::sync::mpsc::TryRecvError;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::oneshot;

use crate::device::Device;
use crate::messaging::com_id::{
    HandleComIdRequest, HandleComIdResponse, HANDLE_COM_ID_PROTOCOL, HANDLE_COM_ID_RESPONSE_LEN,
};
use crate::messaging::packet::{ComPacket, PACKETIZED_PROTOCOL};
use crate::rpc::{Error, PackagedMethod, Properties};
use crate::serialization::{DeserializeBinary, SerializeBinary};

use super::retry::Retry;
use super::rpc_stack::RPCStack;
use super::session_identifier::SessionIdentifier;
use super::tracked::Tracked;

type PacketResponse = Result<PackagedMethod, Error>;
type ComIdResponse = Result<HandleComIdResponse, Error>;
type PacketPromise = oneshot::Sender<PacketResponse>;
type ComIdPromise = oneshot::Sender<ComIdResponse>;

pub enum Message {
    Method { session: SessionIdentifier, content: Tracked<PackagedMethod, PacketResponse> },
    HandleComId { content: Tracked<HandleComIdRequest, ComIdResponse> },
    StartSession { session: SessionIdentifier, properties: Properties },
    EndSession { session: SessionIdentifier },
}

pub struct MessageLoop {
    messages: std::sync::mpsc::Receiver<Message>,
    device: Arc<dyn Device>,
    stack: RPCStack,
    properties: Properties,
}

impl MessageLoop {
    pub fn new(
        messages: std::sync::mpsc::Receiver<Message>,
        device: Arc<dyn Device>,
        stack: RPCStack,
        properties: Properties,
    ) -> Self {
        Self { messages, device, stack, properties }
    }

    pub fn run(mut self) {
        loop {
            let (messages, connected) = self.recv_messages();
            messages.into_iter().for_each(|message| self.enqueue_message(message));
            if let Some(request) = self.stack.poll_com_id() {
                self.exchange_com_id(request);
            } else if let Some((com_packet, promise_groups)) = self.stack.poll_packet() {
                self.exchange_packet(com_packet, promise_groups);
            } else {
                if !connected {
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            self.stack.forward_results();
            self.stack.remove_sessions();
        }
    }

    fn recv_messages(&self) -> (Vec<Message>, bool) {
        let mut messages = Vec::new();
        let connected = loop {
            match self.messages.try_recv() {
                Ok(message) => messages.push(message),
                Err(TryRecvError::Disconnected) => break false,
                Err(TryRecvError::Empty) => break true,
            }
        };
        (messages, connected)
    }

    fn enqueue_message(&mut self, message: Message) {
        match message {
            Message::Method { session, content } => self.stack.send_packet(session, content),
            Message::HandleComId { content } => self.stack.send_com_id(content),
            Message::StartSession { session, properties } => self.stack.insert_session(session, properties),
            Message::EndSession { session } => self.stack.remove_session(session),
        }
    }

    fn exchange_com_id(&mut self, request: Tracked<HandleComIdRequest, ComIdResponse>) {
        let _ = self.send_com_id(request);
        if let Ok(response) = self.recv_com_id() {
            self.stack.recv_com_id(response);
        }
    }

    fn exchange_packet(
        &mut self,
        com_packet: ComPacket,
        promise_groups: Vec<Tracked<SessionIdentifier, Result<PackagedMethod, Error>>>,
    ) {
        let _ = self.send_packet(com_packet, promise_groups);
        if let Ok(com_packet) = self.recv_packet() {
            self.stack.recv_packet(com_packet);
        }
    }

    fn send_packet(
        &mut self,
        com_packet: ComPacket,
        promise_groups: Vec<Tracked<SessionIdentifier, Result<PackagedMethod, Error>>>,
    ) -> Result<(), Error> {
        let data = com_packet.to_bytes().map_err(|err| Error::SerializationFailed(err))?;
        let result = self.device.security_send(PACKETIZED_PROTOCOL, self.stack.com_id().to_be_bytes(), &data);
        match result {
            Ok(_) => {
                for Tracked { item, promises } in promise_groups {
                    for promise in promises {
                        self.stack.return_promise_packet(item, promise);
                    }
                }
                Ok(())
            }
            Err(err) => {
                for tracked in promise_groups {
                    tracked.close(Err(Error::SecuritySendFailed(err.clone())));
                }
                Err(Error::SecuritySendFailed(err))
            }
        }
    }

    fn send_com_id(&mut self, request: Tracked<HandleComIdRequest, ComIdResponse>) -> Result<(), Error> {
        let Tracked { item: request, promises } = request;
        let data = request.to_bytes().map_err(|err| Error::SerializationFailed(err))?;
        let result = self.device.security_send(HANDLE_COM_ID_PROTOCOL, self.stack.com_id().to_be_bytes(), &data);
        match result {
            Ok(_) => {
                for promise in promises {
                    self.stack.return_promise_com_id(promise);
                }
                Ok(())
            }
            Err(err) => {
                Tracked { item: (), promises }.close(Err(Error::SecuritySendFailed(err.clone())));
                Err(Error::SecuritySendFailed(err.clone()))
            }
        }
    }

    fn recv_packet(&mut self) -> Result<ComPacket, Error> {
        let mut retry = Retry::new(self.properties.trans_timeout);
        let mut com_packet = ComPacket::default();
        loop {
            let transfer_len = self.optimal_transfer_len(com_packet.min_transfer, com_packet.outstanding_data);
            match self.recv_partial_packet(transfer_len) {
                Ok(new_com_packet) => com_packet.append(new_com_packet),
                Err(err) => break Err(err),
            }
            if com_packet.outstanding_data != 0 {
                match retry.sleep() {
                    Ok(_) => (),
                    Err(err) => break Err(err),
                }
            } else {
                break Ok(com_packet);
            }
        }
    }

    fn recv_partial_packet(&self, transfer_len: usize) -> Result<ComPacket, Error> {
        let protocol_specific = self.stack.com_id().to_be_bytes();
        let data = self
            .device
            .security_recv(PACKETIZED_PROTOCOL, protocol_specific, transfer_len)
            .map_err(|err| Error::SecurityReceiveFailed(err))?;
        ComPacket::from_bytes(data).map_err(|err| Error::SerializationFailed(err))
    }

    fn optimal_transfer_len(&self, min_transfer: u32, outstanding_data: u32) -> usize {
        let limit = self.properties.max_gross_compacket_response_size;
        let desired = std::cmp::max(512, std::cmp::min(limit, outstanding_data as usize));
        std::cmp::max(min_transfer as usize, desired)
    }

    fn recv_com_id(&mut self) -> Result<HandleComIdResponse, Error> {
        let mut retry = Retry::new(self.properties.trans_timeout);
        loop {
            let transfer_len = HANDLE_COM_ID_RESPONSE_LEN;
            let protocol_specific = self.stack.com_id().to_be_bytes();
            let data = self
                .device
                .security_recv(HANDLE_COM_ID_PROTOCOL, protocol_specific, transfer_len)
                .map_err(|err| Error::SecurityReceiveFailed(err))?;
            let response = HandleComIdResponse::from_bytes(data).map_err(|err| Error::SerializationFailed(err))?;
            if response.payload.is_empty() {
                match retry.sleep() {
                    Ok(_) => (),
                    Err(err) => break Err(err),
                }
            } else {
                break Ok(response);
            }
        }
    }
}

pub fn message_loop(messages: std::sync::mpsc::Receiver<Message>, device: Arc<dyn Device>, stack: RPCStack) {
    MessageLoop::new(messages, device, stack, Properties::ASSUMED).run();
}
