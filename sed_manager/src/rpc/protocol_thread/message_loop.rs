use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

use tokio::sync::oneshot;

use crate::device::Device;
use crate::messaging::com_id::{
    HandleComIdRequest, HandleComIdResponse, HANDLE_COM_ID_PROTOCOL, HANDLE_COM_ID_RESPONSE_LEN,
};
use crate::messaging::packet::{ComPacket, PACKETIZED_PROTOCOL};
use crate::rpc::{Error, PackagedMethod, Properties};
use crate::serialization::{DeserializeBinary, SerializeBinary};

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

pub fn message_loop(messages: std::sync::mpsc::Receiver<Message>, device: Box<dyn Device>, mut stack: RPCStack) {
    loop {
        match messages.recv_timeout(Duration::from_millis(200)) {
            Ok(message) => match message {
                Message::Method { session, content } => stack.send_packet(session, content),
                Message::HandleComId { content } => stack.send_com_id(content),
                Message::StartSession { session, properties } => stack.insert_session(session, properties),
                Message::EndSession { session } => stack.remove_session(session),
            },
            Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => (),
        }

        if let Some(Tracked { item: request, promises }) = stack.poll_com_id() {
            let response = sync_com_id_request(&*device, stack.com_id(), request);
            match response {
                Ok(response) => {
                    promises.into_iter().for_each(|pr| stack.return_promise_com_id(pr));
                    stack.recv_com_id(response);
                }
                Err(err) => Tracked::new((), promises).close(Err(err)),
            }
        } else if let Some((com_packet, trackers)) = stack.poll_packet() {
            let response = sync_packet_request(&*device, stack.com_id(), com_packet);
            match response {
                Ok(response) => {
                    trackers.into_iter().for_each(|Tracked { item: session, promises }| {
                        promises.into_iter().for_each(|pr| stack.return_promise_packet(session, pr));
                    });
                    stack.recv_packet(response);
                }
                Err(err) => trackers.into_iter().for_each(|tracker| tracker.close(Err(err.clone()))),
            }
        }

        stack.forward_results();
        stack.remove_sessions();
    }
}

fn sync_com_id_request(
    device: &dyn Device,
    com_id: u16,
    request: HandleComIdRequest,
) -> Result<HandleComIdResponse, Error> {
    let data = request.to_bytes().map_err(|err| Error::SerializationFailed(err))?;
    device
        .security_send(HANDLE_COM_ID_PROTOCOL, com_id.to_be_bytes(), &data)
        .map_err(|err| Error::SecuritySendFailed(err))?;
    for _ in 0..20 {
        let result = device.security_recv(HANDLE_COM_ID_PROTOCOL, com_id.to_be_bytes(), HANDLE_COM_ID_RESPONSE_LEN);
        if let Ok(data) = result {
            let response = HandleComIdResponse::from_bytes(data).map_err(|err| Error::SerializationFailed(err))?;
            if !response.payload.is_empty() {
                return Ok(response);
            }
        }
    }
    Err(Error::TimedOut)
}

fn sync_packet_request(device: &dyn Device, com_id: u16, request: ComPacket) -> Result<ComPacket, Error> {
    let data = request.to_bytes().map_err(|err| Error::SerializationFailed(err))?;
    device
        .security_send(PACKETIZED_PROTOCOL, com_id.to_be_bytes(), &data)
        .map_err(|err| Error::SecuritySendFailed(err))?;
    let mut transfer_len = 512;
    let mut packets = Vec::new();
    for _ in 0..20 {
        let result = device.security_recv(PACKETIZED_PROTOCOL, com_id.to_be_bytes(), HANDLE_COM_ID_RESPONSE_LEN);
        if let Ok(data) = result {
            let response = ComPacket::from_bytes(data).map_err(|err| Error::SerializationFailed(err))?;
            packets.append(&mut response.payload.into_vec());
            let response = ComPacket { payload: std::mem::replace(&mut packets, Vec::new()).into(), ..response };
            transfer_len = std::cmp::max(transfer_len, response.min_transfer);
            if response.outstanding_data == 0 {
                return Ok(response);
            }
        }
    }
    Err(Error::TimedOut)
}
