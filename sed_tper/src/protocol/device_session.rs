use core::mem::replace;
use std::cmp::{max, min};
use std::time::Instant;
use std::{collections::VecDeque, sync::Arc};

use sed_device::Device;
use sed_packet::com_id::{COM_ID_RESPONSE_LEN, ComIdResponse, ComIdResponsePayload};
use sed_packet::packet::ComPacket;
use sed_packet::session_id::SessionId;
use sed_spec::methods::Properties;
use sorbit::ser_de::{FromBytes, ToBytes};
use tracing::{Instrument as _, debug, debug_span, instrument};

use crate::error::Error;
use crate::protocol::message::{
    ComResponse, ComResponseReceived, CommitBatch, Message, PacketReceived, SecuritySendDone, SendComRequest,
    SendComRequestDone, SendPacket, SendPacketDone,
};
use crate::protocol::method::WriteQueuedMethod;
use crate::protocol::protocol::{Address, Context};
use crate::protocol::retry::Retry;

pub struct DeviceSession {
    com_id: u16,
    com_id_ext: u16,
    device: Arc<dyn Device>,
    packet_queue: VecDeque<SendPacket>,
    packet_state: PacketProtocolState,
    com_id_queue: VecDeque<SendComRequest>,
    com_id_state: ComIdProtocolState,
}

impl DeviceSession {
    const ADDRESS: Address = Address::DeviceSession;

    pub fn new(com_id: u16, com_id_ext: u16, device: Arc<dyn Device>) -> Self {
        Self {
            com_id,
            com_id_ext,
            device,
            packet_queue: VecDeque::new(),
            packet_state: PacketProtocolState::Ready,
            com_id_queue: VecDeque::new(),
            com_id_state: ComIdProtocolState::Ready,
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn send_packet(&mut self, context: Context, message: SendPacket) {
        self.packet_queue.push_back(message);
        context.send(Self::ADDRESS, Message::CommitBatch(CommitBatch));
    }

    #[instrument(level = "debug", skip_all)]
    pub fn send_com_request(&mut self, context: Context, message: SendComRequest) {
        self.com_id_queue.push_back(message);
        context.send(Self::ADDRESS, Message::CommitBatch(CommitBatch));
    }

    #[instrument(level = "debug", skip_all)]
    pub fn commit_batch(&mut self, context: Context, _message: CommitBatch) {
        self.commit_batch_com_packet(context.clone());
        self.commit_batch_com_id_request(context);
    }

    #[instrument(level = "debug", skip_all)]
    pub fn security_send_done(&mut self, context: Context, SecuritySendDone { protocol, result }: SecuritySendDone) {
        match protocol {
            0x01 => self.security_send_done_com_packet(context, result),
            0x02 => self.security_send_done_com_id_request(context, result),
            0x03 => unimplemented!("security protocol 0x03 is not supported"),
            _ => unreachable!("should not attempts security send on invalid protocols"),
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn security_recv_com_packet_done(&mut self, context: Context, result: Result<ComPacket, Error>) {
        self.packet_state = match replace(&mut self.packet_state, PacketProtocolState::Processing) {
            PacketProtocolState::Receiving => match result {
                Ok(com_packet) => {
                    for packet in com_packet.payload {
                        let session_id = SessionId::of(&packet);
                        let address = Address::from(session_id);
                        context.send(address, Message::PacketReceived(PacketReceived { packet }));
                    }
                    PacketProtocolState::Ready
                }
                Err(_) => PacketProtocolState::Ready,
            },
            state => state,
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn security_recv_com_id_request_done(&mut self, context: Context, result: Result<ComIdResponse, Error>) {
        self.com_id_state = match replace(&mut self.com_id_state, ComIdProtocolState::Processing) {
            ComIdProtocolState::Receiving => match result {
                Ok(response) => {
                    context.send(Address::ComSession, Message::ComResponseReceived(ComResponseReceived { response }));
                    ComIdProtocolState::Ready
                }
                Err(_) => ComIdProtocolState::Ready,
            },
            state => state,
        }
    }

    #[instrument(level = "debug", skip_all)]
    fn commit_batch_com_packet(&mut self, context: Context) {
        self.packet_state = match replace(&mut self.packet_state, PacketProtocolState::Processing) {
            PacketProtocolState::Ready => {
                if let Some(SendPacket { sender, packet, methods }) = self.packet_queue.pop_front() {
                    let com_packet = ComPacket {
                        com_id: self.com_id,
                        com_id_ext: self.com_id_ext,
                        payload: vec![packet],
                        ..Default::default()
                    };
                    let data = com_packet.to_bytes().expect("should not normally fail, but replace it with a check");
                    context.send_future(Self::ADDRESS, security_send(self.device.clone(), 0x01, self.com_id, data));
                    PacketProtocolState::Sending { sender, methods }
                } else {
                    PacketProtocolState::Ready
                }
            }
            state => state,
        }
    }

    #[instrument(level = "debug", skip_all)]
    fn commit_batch_com_id_request(&mut self, context: Context) {
        self.com_id_state = match replace(&mut self.com_id_state, ComIdProtocolState::Processing) {
            ComIdProtocolState::Ready => {
                if let Some(SendComRequest { request, channel }) = self.com_id_queue.pop_front() {
                    let data = request.to_bytes().expect("should not normally fail, but replace it with a check");
                    context.send_future(Self::ADDRESS, security_send(self.device.clone(), 0x02, self.com_id, data));
                    ComIdProtocolState::Sending { channel }
                } else {
                    ComIdProtocolState::Ready
                }
            }
            state => state,
        }
    }

    #[instrument(level = "debug", skip_all)]
    fn security_send_done_com_packet(&mut self, context: Context, result: Result<(), sed_device::Error>) {
        self.packet_state = match replace(&mut self.packet_state, PacketProtocolState::Processing) {
            PacketProtocolState::Sending { sender, methods } => match result {
                Ok(_) => {
                    context.send(sender, Message::SendPacketDone(SendPacketDone { status: Ok(()), methods }));
                    context.send_future(Self::ADDRESS, security_recv_com_packet(self.device.clone(), self.com_id));
                    PacketProtocolState::Receiving
                }
                Err(err) => {
                    context.send(sender, Message::SendPacketDone(SendPacketDone { status: Err(err.into()), methods }));
                    PacketProtocolState::Ready
                }
            },
            state => state,
        }
    }

    #[instrument(level = "debug", skip_all)]
    fn security_send_done_com_id_request(&mut self, context: Context, result: Result<(), sed_device::Error>) {
        let sender = Address::ComSession;
        self.com_id_state = match replace(&mut self.com_id_state, ComIdProtocolState::Processing) {
            ComIdProtocolState::Sending { channel } => match result {
                Ok(_) => {
                    let status = Ok(());
                    context.send(sender, Message::SendComRequestDone(SendComRequestDone { status, channel }));
                    context.send_future(Self::ADDRESS, security_recv_com_id_request(self.device.clone(), self.com_id));
                    ComIdProtocolState::Receiving
                }
                Err(err) => {
                    let status = Err(err.into());
                    context.send(sender, Message::SendComRequestDone(SendComRequestDone { status, channel }));
                    ComIdProtocolState::Ready
                }
            },
            state => state,
        }
    }
}

impl core::fmt::Debug for DeviceSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceSession")
            .field("com_id", &self.com_id)
            .field("com_id_ext", &self.com_id_ext)
            .field("device", &format!("{} ({})", self.device.model_number(), self.device.serial_number()))
            .field("packet_queue", &self.packet_queue)
            .field("packet_state", &self.packet_state)
            .field("com_id_queue", &self.com_id_queue)
            .field("com_id_state", &self.com_id_state)
            .finish()
    }
}

#[derive(Debug)]
enum PacketProtocolState {
    Processing,
    Ready,
    Sending { sender: Address, methods: Vec<WriteQueuedMethod> },
    Receiving,
}

#[derive(Debug)]
enum ComIdProtocolState {
    Processing,
    Ready,
    Sending { channel: oneshot::Sender<ComResponse> },
    Receiving,
}

async fn security_send(device: Arc<dyn Device>, protocol: u8, com_id: u16, data: Vec<u8>) -> Message {
    let result = device.security_send(protocol, com_id.to_be_bytes(), &data).await;
    Message::SecuritySendDone(SecuritySendDone { protocol, result })
}

async fn security_recv_com_packet(device: Arc<dyn Device>, com_id: u16) -> Message {
    #[instrument(level = "debug", skip_all)]
    async fn _security_recv_com_packet(device: Arc<dyn Device>, com_id: u16) -> Result<ComPacket, Error> {
        let mut retry = Retry::new(Instant::now() + Properties::ASSUMED.def_trans_timeout);
        let mut transfer_len = 1024;
        let mut merged = ComPacket::default();
        loop {
            let bytes = device.security_recv(0x01, com_id.to_be_bytes(), transfer_len).await?;
            let response = ComPacket::from_bytes(&bytes).map_err(|err| Error::InvalidComIdResponse(err))?;
            debug!(response = ?response);
            let outstanding_data = response.outstanding_data;
            transfer_len = min(max(response.min_transfer, outstanding_data), 256 * 1024) as usize;
            merged.append(response);
            if outstanding_data == 0 {
                break Ok(merged);
            } else if outstanding_data == 1 {
                retry.sleep().instrument(debug_span!("sleep")).await?;
            }
        }
    }
    Message::SecurityRecvDoneComPacket(_security_recv_com_packet(device, com_id).await)
}

async fn security_recv_com_id_request(device: Arc<dyn Device>, com_id: u16) -> Message {
    #[instrument(level = "debug", skip_all)]
    async fn _security_recv_com_id_request(device: Arc<dyn Device>, com_id: u16) -> Result<ComIdResponse, Error> {
        let mut retry = Retry::new(Instant::now() + Properties::ASSUMED.def_trans_timeout);
        loop {
            let bytes = device.security_recv(0x02, com_id.to_be_bytes(), COM_ID_RESPONSE_LEN).await?;
            let response = ComIdResponse::from_bytes(&bytes).map_err(|err| Error::InvalidComIdResponse(err))?;
            match &response.payload {
                ComIdResponsePayload::NoResponseAvailable { .. } => retry.sleep().await?,
                ComIdResponsePayload::Verify { .. } => break Ok(response),
                ComIdResponsePayload::StackReset { .. } => break Ok(response),
            }
        }
    }
    Message::SecurityRecvDoneComIdRequest(_security_recv_com_id_request(device, com_id).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::marker::PhantomData;

    use crate::protocol::message::{PacketReceived, SendPacket};
    use crate::protocol::protocol::DispatchMessage;

    use sed_device::Error as DeviceError;
    use sed_device::mock_device::{MockDevice, MockEvent};
    use sed_packet::com_id::{ComIdRequest, StackResetStatus};
    use sed_packet::packet::{Packet, SubPacket, SubPacketKind};

    fn create_request_com_packet() -> ComPacket {
        ComPacket {
            com_id: 0x0001,
            com_id_ext: 0x0000,
            payload: vec![Packet {
                tper_session_number: 1,
                host_session_number: 2,
                payload: vec![SubPacket { kind: SubPacketKind::Data, length: PhantomData, payload: vec![0x01] }],
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    fn create_reply_com_packet() -> ComPacket {
        ComPacket {
            com_id: 0x0001,
            com_id_ext: 0x0000,
            payload: vec![Packet {
                tper_session_number: 1,
                host_session_number: 2,
                payload: vec![SubPacket { kind: SubPacketKind::Data, length: PhantomData, payload: vec![0x02] }],
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn send_packet_sequence() {
        // This is a rather lengthy test, but it probably makes more sense to
        // test the entire process than to manually set the state machine

        let request_com_packet = create_request_com_packet();
        let reply_com_packet = create_reply_com_packet();

        let events = [
            MockEvent::Send {
                name: Some("send method call".into()),
                security_protocol: 0x01,
                protocol_specific: 0x0001_u16.to_be_bytes(),
                expected: request_com_packet.to_bytes().unwrap(),
                result: Ok(()),
            },
            MockEvent::Recv {
                name: Some("recv method result".into()),
                security_protocol: 0x01,
                protocol_specific: 0x0001_u16.to_be_bytes(),
                result: Ok(reply_com_packet.to_bytes().unwrap()),
            },
        ];

        let device = Arc::new(MockDevice::new(events.into_iter()));
        let mut device_session = DeviceSession::new(0x0001, 0x0000, device.clone());
        let (context, queue) = Context::mock();

        // Issue SendPacket message.
        let session_id = SessionId { hsn: 2, tsn: 1 };
        let sender_address = Address::from(session_id);
        device_session.send_packet(
            context.clone(),
            SendPacket {
                sender: sender_address.clone(),
                packet: request_com_packet.payload[0].clone(),
                methods: vec![],
            },
        );

        // Loop back CommitBatch message.
        assert!(matches!(
            queue.try_recv(),
            Ok(DispatchMessage { address: Address::DeviceSession, message: Message::CommitBatch(_), .. },)
        ));
        device_session.commit_batch(context.clone(), CommitBatch);

        // Loop back SecuritySendDone message.
        // - Let the IF-SEND task run.
        tokio::task::yield_now().await;
        assert!(matches!(
            queue.try_recv(),
            Ok(DispatchMessage { address: Address::DeviceSession, message: Message::SecuritySendDone(_), .. })
        ));
        device_session.security_send_done(context.clone(), SecuritySendDone { protocol: 0x01, result: Ok(()) });

        // Make sure the original session got informed via a SendPacketDone message.
        assert!(matches!(
            queue.try_recv(),
            Ok(DispatchMessage {
                address: Address::Session(SessionId { hsn: 2, tsn: 1 }),
                message: Message::SendPacketDone(_),
                ..
            })
        ));

        // Loop back SecurityRecvComPacketDone message.
        // - Let the IF-RECV task run.
        tokio::task::yield_now().await;
        assert!(matches!(
            queue.try_recv(),
            Ok(DispatchMessage { address: Address::DeviceSession, message: Message::SecurityRecvDoneComPacket(_), .. })
        ));
        device_session.security_recv_com_packet_done(context.clone(), Ok(reply_com_packet));

        // Verify state is back to Ready.
        assert!(matches!(device_session.packet_state, PacketProtocolState::Ready));

        // Make sure the original session got the packet via PacketReceived.
        let DispatchMessage { address, message, .. } = queue.try_recv().expect("should have received PacketReceived");
        assert_eq!(address, Address::Session(session_id));
        match message {
            Message::PacketReceived(PacketReceived { packet }) => {
                assert_eq!(packet.tper_session_number, session_id.tsn);
                assert_eq!(packet.host_session_number, session_id.hsn);
            }
            _ => panic!("expected PacketReceived message but got: {:?}", message),
        }

        device.check();
    }

    #[tokio::test]
    async fn send_packet_interface_send_failure() {
        let request_com_packet = create_request_com_packet();

        let events = [MockEvent::Send {
            name: Some("send method call".into()),
            security_protocol: 0x01,
            protocol_specific: 0x0001_u16.to_be_bytes(),
            expected: request_com_packet.to_bytes().unwrap(),
            result: Err(DeviceError::NotSupported),
        }];

        let device = Arc::new(MockDevice::new(events.into_iter()));
        let mut device_session = DeviceSession::new(0x0001, 0x0000, device.clone());
        let (context, queue) = Context::mock();

        // Enqueue a packet for sending.
        let session_id = SessionId { hsn: 2, tsn: 1 };
        let sender_address = Address::from(session_id);
        device_session.packet_queue.push_back(SendPacket {
            sender: sender_address.clone(),
            packet: request_com_packet.payload[0].clone(),
            methods: vec![],
        });

        // Initiate IF-SEND by committing the packets.
        device_session.commit_batch(context.clone(), CommitBatch);

        // Loop back SecuritySendDone message.
        // - Let the IF-SEND task run.
        tokio::task::yield_now().await;
        assert!(matches!(
            queue.try_recv(),
            Ok(DispatchMessage {
                address: Address::DeviceSession,
                message: Message::SecuritySendDone(SecuritySendDone { result: Err(_), .. }),
                ..
            })
        ));
        device_session.security_send_done(
            context.clone(),
            SecuritySendDone { protocol: 0x01, result: Err(DeviceError::NotSupported) },
        );

        // Make sure the original session got informed via a SendPacketDone message.
        assert!(matches!(
            queue.try_recv(),
            Ok(DispatchMessage {
                address: Address::Session(SessionId { hsn: 2, tsn: 1 }),
                message: Message::SendPacketDone(SendPacketDone { status: Err(_), .. }),
                ..
            })
        ));

        device.check();
    }

    #[tokio::test]
    async fn send_packet_recv_failure() {
        let events = [];
        let device = Arc::new(MockDevice::new(events.into_iter()));
        let mut device_session = DeviceSession::new(0x0001, 0x0000, device.clone());
        let (context, queue) = Context::mock();

        // Set up state for the test.
        device_session.packet_state = PacketProtocolState::Receiving;

        // Simulate a SecurityRecvComPacketDone message.
        device_session.security_recv_com_packet_done(
            context.clone(),
            Err(Error::SecurityCommandFailed(DeviceError::NotSupported)),
        );

        // Verify state is back to Ready.
        assert!(matches!(device_session.packet_state, PacketProtocolState::Ready));

        // Make sure the original session got the packet via PacketReceived.
        assert!(queue.is_empty());

        device.check();
    }

    #[tokio::test]
    async fn send_com_id_sequence() {
        // This is a rather lengthy test, but it probably makes more sense to
        // test the entire process than to manually set the state machine

        let request = ComIdRequest::stack_reset(1, 0);
        let reply = ComIdResponse {
            com_id: 1,
            com_id_ext: 0,
            payload: ComIdResponsePayload::StackReset { available_data_length: 0, status: StackResetStatus::Success },
        };

        let events = [
            MockEvent::Send {
                name: Some("send method call".into()),
                security_protocol: 0x02,
                protocol_specific: 0x0001_u16.to_be_bytes(),
                expected: request.to_bytes().unwrap(),
                result: Ok(()),
            },
            MockEvent::Recv {
                name: Some("recv method result".into()),
                security_protocol: 0x02,
                protocol_specific: 0x0001_u16.to_be_bytes(),
                result: Ok(reply.to_bytes().unwrap()),
            },
        ];

        let device = Arc::new(MockDevice::new(events.into_iter()));
        let mut device_session = DeviceSession::new(0x0001, 0x0000, device.clone());
        let (channel, _) = oneshot::channel();
        let (context, queue) = Context::mock();

        // Issue SendComRequest message.
        device_session.send_com_request(context.clone(), SendComRequest { request, channel });

        // Loop back CommitBatch message.
        assert!(matches!(
            queue.try_recv(),
            Ok(DispatchMessage { address: Address::DeviceSession, message: Message::CommitBatch(_), .. })
        ));
        device_session.commit_batch(context.clone(), CommitBatch);

        // Loop back SecuritySendDone message.
        // - Let the IF-SEND task run.
        tokio::task::yield_now().await;
        let message = queue.try_recv();
        assert!(
            matches!(
                message,
                Ok(DispatchMessage { address: Address::DeviceSession, message: Message::SecuritySendDone(_), .. })
            ),
            "{message:?}"
        );
        device_session.security_send_done(context.clone(), SecuritySendDone { protocol: 0x02, result: Ok(()) });

        // Make sure the original session got informed via a SendComRequestDone message.
        let message = queue.try_recv();
        assert!(
            matches!(
                message,
                Ok(DispatchMessage { address: Address::ComSession, message: Message::SendComRequestDone(_), .. })
            ),
            "{message:?}"
        );

        // Loop back SecurityRecvDoneComIdRequest message.
        // - Let the IF-RECV task run.
        tokio::task::yield_now().await;
        assert!(matches!(
            queue.try_recv(),
            Ok(DispatchMessage {
                address: Address::DeviceSession,
                message: Message::SecurityRecvDoneComIdRequest(_),
                ..
            })
        ));
        device_session.security_recv_com_id_request_done(context.clone(), Ok(reply.clone()));

        // Verify state is back to Ready.
        assert!(
            matches!(device_session.com_id_state, ComIdProtocolState::Ready),
            "{:?}",
            device_session.com_id_state
        );

        // Make sure the original session got the packet via PacketReceived.
        let DispatchMessage { address, message, .. } =
            queue.try_recv().expect("should have received ComResponseReceived");
        assert_eq!(address, Address::ComSession);
        match message {
            Message::ComResponseReceived(ComResponseReceived { response: response_ }) => {
                assert_eq!(response_, reply);
            }
            _ => panic!("expected ComResponseReceived message but got: {:?}", message),
        }

        device.check();
    }

    #[tokio::test]
    async fn send_com_id_interface_send_failure() {
        let request = ComIdRequest::stack_reset(1, 0);

        let events = [MockEvent::Send {
            name: Some("send method call".into()),
            security_protocol: 0x02,
            protocol_specific: 0x0001_u16.to_be_bytes(),
            expected: request.to_bytes().unwrap(),
            result: Err(DeviceError::NotSupported),
        }];

        let device = Arc::new(MockDevice::new(events.into_iter()));
        let mut device_session = DeviceSession::new(0x0001, 0x0000, device.clone());
        let (channel, _) = oneshot::channel();
        let (context, queue) = Context::mock();

        // Issue SendComRequest message.
        device_session.send_com_request(context.clone(), SendComRequest { request, channel });

        // Loop back CommitBatch message.
        assert!(matches!(
            queue.try_recv(),
            Ok(DispatchMessage { address: Address::DeviceSession, message: Message::CommitBatch(_), .. })
        ));
        device_session.commit_batch(context.clone(), CommitBatch);

        // Loop back SecuritySendDone message.
        // - Let the IF-SEND task run.
        tokio::task::yield_now().await;
        let message = queue.try_recv();
        assert!(
            matches!(
                message,
                Ok(DispatchMessage { address: Address::DeviceSession, message: Message::SecuritySendDone(_), .. })
            ),
            "{message:?}"
        );
        device_session.security_send_done(
            context.clone(),
            SecuritySendDone { protocol: 0x02, result: Err(DeviceError::NotSupported) },
        );

        // Make sure the original session got informed via a SendComRequestDone message.
        let message = queue.try_recv();
        assert!(
            matches!(
                message,
                Ok(DispatchMessage {
                    address: Address::ComSession,
                    message: Message::SendComRequestDone(SendComRequestDone { status: Err(_), .. }),
                    ..
                })
            ),
            "{message:?}"
        );

        device.check();
    }

    #[tokio::test]
    async fn send_com_id_recv_failure() {
        let events = [];
        let device = Arc::new(MockDevice::new(events.into_iter()));
        let mut device_session = DeviceSession::new(0x0001, 0x0000, device.clone());
        let (context, queue) = Context::mock();

        // Set up state for the test.
        device_session.com_id_state = ComIdProtocolState::Receiving;

        // Simulate a SecurityRecvComIdRequestDone message.
        device_session.security_recv_com_id_request_done(
            context.clone(),
            Err(Error::SecurityCommandFailed(DeviceError::NotSupported)),
        );

        // Verify state is back to Ready.
        assert!(matches!(device_session.com_id_state, ComIdProtocolState::Ready));

        // Make sure the original session got the packet via PacketReceived.
        assert!(queue.is_empty());

        device.check();
    }
}
