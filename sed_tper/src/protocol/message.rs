use std::time::Instant;

use sed_device::Error as DeviceError;
use sed_packet::com_id::{HandleComIdRequest, HandleComIdResponse};
use sed_packet::packet::{ComPacket, Packet};
use sed_spec::methods::Properties;
use tracing::Span;

use crate::error::Error;
use crate::protocol::method::WriteQueuedMethod;
use crate::protocol::protocol::Address;
use crate::protocol::session_id::SessionId;

pub type MethodResponse = Result<Vec<u8>, Error>;
pub type ComResponse = Result<HandleComIdResponse, Error>;

#[non_exhaustive]
#[derive(Debug)]
pub enum Message {
    SendMethod(SendMethod),
    SendComRequest(SendComRequest),
    CommitBatch(CommitBatch),
    Abort(Abort),
    Spawn(Spawn),
    Delete(Delete),
    SendPacket(SendPacket),
    SendPacketDone(SendPacketDone),
    SendComRequestDone(SendComRequestDone),
    PacketReceived(PacketReceived),
    ComResponseReceived(ComResponseReceived),
    Timeout(Instant),
    SecuritySendDone(SecuritySendDone),
    SecurityRecvDoneComPacket(Result<ComPacket, Error>),
    SecurityRecvDoneComIdRequest(Result<HandleComIdResponse, Error>),
    ContextDropped,
    Shutdown(oneshot::Sender<Result<(), Error>>, Instant),
}

/// Initiate an RPC method call to the device.
#[derive(Debug)]
pub struct SendMethod {
    pub method: Vec<u8>,
    pub channel: oneshot::Sender<MethodResponse>,
    pub span: Span,
}

/// Initiate a ComID request to the device.
#[derive(Debug)]
pub struct SendComRequest {
    pub request: HandleComIdRequest,
    pub channel: oneshot::Sender<ComResponse>,
    pub span: Span,
}

/// Send buffered content to the device immediately.
///
/// Some messages, like methods can be buffered and sent together to the device.
/// This requires fewer IOCTL calls and reduces overhead. However, at some point
/// the data has to eventually be sent, which is requested by this message.
#[derive(Debug)]
pub struct CommitBatch(pub Span);

/// The session receiving this should abort itself.
///
/// In the address, you have to specify the TSN and the HSN of the session that
/// should be aborted.
#[derive(Debug)]
pub struct Abort;

/// Spawn a new session by creating its data structure.
#[derive(Debug, PartialEq, Eq)]
pub struct Spawn {
    pub id: SessionId,
    pub properties: Properties,
}

/// Delete the data structures associated with the session.
///
/// This should be sent to the control address by the session that wishes to
/// delete itself.
#[derive(Debug)]
pub struct Delete(pub SessionId);

/// Send a packet to the device.
///
/// The packet will be wrapped in a ComPacket.
#[derive(Debug)]
pub struct SendPacket {
    pub sender: Address,
    pub packet: Packet,
    pub methods: Vec<WriteQueuedMethod>,
}

/// An event that is emitted when sending a packet to the device is done.
#[derive(Debug)]
pub struct SendPacketDone {
    pub status: Result<(), Error>,
    pub methods: Vec<WriteQueuedMethod>,
}

/// An event that is emitted when sending a ComID request to the device is done.
#[derive(Debug)]
pub struct SendComRequestDone {
    pub status: Result<(), Error>,
    pub channel: oneshot::Sender<ComResponse>,
    pub span: Span,
}

/// An event that is emitted when a packet is received from the device.
#[derive(Debug)]
pub struct PacketReceived {
    pub packet: Packet,
}

/// An event that is emitted when a ComID response is received from the device.
#[derive(Debug)]
pub struct ComResponseReceived {
    pub response: HandleComIdResponse,
}

#[derive(Debug)]
pub struct SecuritySendDone {
    pub protocol: u8,
    pub result: Result<(), DeviceError>,
}
