use core::mem::replace;
use std::cmp::{max, min};
use std::time::Instant;
use std::{collections::VecDeque, sync::Arc};

use sed_device::Device;
use sed_packet::com_id::{HANDLE_COM_ID_RESPONSE_LEN, HandleComIdResponseParams};
use sed_packet::{com_id::HandleComIdResponse, packet::ComPacket};
use sed_spec::methods::Properties;
use sorbit::ser_de::{FromBytes, ToBytes};
use tracing::Span;

use crate::error::Error;
use crate::protocol::message::{
    ComResponseReceived, ComResult, CommitBatch, Message, MethodResult, PacketReceived, SecuritySendDone,
    SendComRequest, SendComRequestDone, SendPacket, SendPacketDone,
};
use crate::protocol::method::MethodCallPlaceholder;
use crate::protocol::protocol::{Address, Context};
use crate::protocol::retry::Retry;
use crate::protocol::session_id::SessionId;

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

    pub fn send_packet(&mut self, context: Context, message: SendPacket) {
        self.packet_queue.push_back(message);
        context.send(Self::ADDRESS, Message::CommitBatch(CommitBatch));
    }

    pub fn send_com_request(&mut self, context: Context, message: SendComRequest) {
        self.com_id_queue.push_back(message);
        context.send(Self::ADDRESS, Message::CommitBatch(CommitBatch));
    }

    pub fn commit_batch(&mut self, context: Context) {
        self.commit_batch_com_packet(context.clone());
        self.commit_batch_com_id_request(context);
    }

    pub fn security_send_done(&mut self, context: Context, SecuritySendDone { protocol, result }: SecuritySendDone) {
        match protocol {
            0x01 => self.security_send_done_com_packet(context, result),
            0x02 => self.security_send_done_com_id_request(context, result),
            0x03 => unimplemented!("security protocol 0x03 is not supported"),
            _ => unreachable!("should not attempts security send on invalid protocols"),
        }
    }

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

    pub fn security_recv_com_id_request_done(&mut self, context: Context, result: Result<HandleComIdResponse, Error>) {
        self.packet_state = match replace(&mut self.packet_state, PacketProtocolState::Processing) {
            PacketProtocolState::Receiving => match result {
                Ok(response) => {
                    context.send(Address::ComSession, Message::ComResponseReceived(ComResponseReceived { response }));
                    PacketProtocolState::Ready
                }
                Err(_) => PacketProtocolState::Ready,
            },
            state => state,
        }
    }

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

    fn commit_batch_com_id_request(&mut self, context: Context) {
        self.com_id_state = match replace(&mut self.com_id_state, ComIdProtocolState::Processing) {
            ComIdProtocolState::Ready => {
                if let Some(SendComRequest { request, channel, span }) = self.com_id_queue.pop_front() {
                    let data = request.to_bytes().expect("should not normally fail, but replace it with a check");
                    context.send_future(Self::ADDRESS, security_send(self.device.clone(), 0x01, self.com_id, data));
                    ComIdProtocolState::Sending { channel, span }
                } else {
                    ComIdProtocolState::Ready
                }
            }
            state => state,
        }
    }

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

    fn security_send_done_com_id_request(&mut self, context: Context, result: Result<(), sed_device::Error>) {
        let sender = Address::ComSession;
        self.com_id_state = match replace(&mut self.com_id_state, ComIdProtocolState::Processing) {
            ComIdProtocolState::Sending { channel, span } => match result {
                Ok(_) => {
                    let status = Ok(());
                    context.send(sender, Message::SendComRequestDone(SendComRequestDone { status, channel, span }));
                    context.send_future(Self::ADDRESS, security_recv_com_id_request(self.device.clone(), self.com_id));
                    ComIdProtocolState::Receiving
                }
                Err(err) => {
                    let status = Err(err.into());
                    context.send(sender, Message::SendComRequestDone(SendComRequestDone { status, channel, span }));
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
    Sending { sender: Address, methods: Vec<(oneshot::Sender<MethodResult>, Span, MethodCallPlaceholder)> },
    Receiving,
}

#[derive(Debug)]
enum ComIdProtocolState {
    Processing,
    Ready,
    Sending { channel: oneshot::Sender<ComResult>, span: Span },
    Receiving,
}

async fn security_send(device: Arc<dyn Device>, protocol: u8, com_id: u16, data: Vec<u8>) -> Message {
    let result = device.security_send(protocol, com_id.to_be_bytes(), &data);
    Message::SecuritySendDone(SecuritySendDone { protocol, result })
}

async fn security_recv_com_packet(device: Arc<dyn Device>, com_id: u16) -> Message {
    async fn _security_recv_com_packet(device: Arc<dyn Device>, com_id: u16) -> Result<ComPacket, Error> {
        let mut retry = Retry::new(Instant::now() + Properties::ASSUMED.def_trans_timeout);
        let mut transfer_len = 1024;
        let mut merged = ComPacket::default();
        loop {
            let bytes = device.security_recv(0x01, com_id.to_be_bytes(), transfer_len)?;
            let response = ComPacket::from_bytes(&bytes).map_err(|err| Error::InvalidComIdResponse(err))?;
            let outstanding_data = response.outstanding_data;
            transfer_len = min(max(response.min_transfer, outstanding_data), 256 * 1024) as usize;
            merged.append(response);
            if outstanding_data == 0 {
                break Ok(merged);
            } else if outstanding_data == 1 {
                retry.sleep().await?;
            }
        }
    }
    Message::SecurityRecvDoneComPacket(_security_recv_com_packet(device, com_id).await)
}

async fn security_recv_com_id_request(device: Arc<dyn Device>, com_id: u16) -> Message {
    async fn _security_recv_com_id_request(device: Arc<dyn Device>, com_id: u16) -> Result<HandleComIdResponse, Error> {
        let mut retry = Retry::new(Instant::now() + Properties::ASSUMED.def_trans_timeout);
        loop {
            let bytes = device.security_recv(0x02, com_id.to_be_bytes(), HANDLE_COM_ID_RESPONSE_LEN)?;
            let response = HandleComIdResponse::from_bytes(&bytes).map_err(|err| Error::InvalidComIdResponse(err))?;
            match &response.params {
                HandleComIdResponseParams::NoResponseAvailable { .. } => retry.sleep().await?,
                HandleComIdResponseParams::VerifyComIdValid { .. } => break Ok(response),
                HandleComIdResponseParams::StackReset { .. } => break Ok(response),
            }
        }
    }
    Message::SecurityRecvDoneComIdRequest(_security_recv_com_id_request(device, com_id).await)
}
