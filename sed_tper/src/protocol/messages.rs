use sed_packet::com_id::{HandleComIdRequest, HandleComIdResponse};
use sed_packet::packet::Packet;
use tracing::Span;

use crate::error::Error;
use crate::protocol::method_structure::MethodCallPlaceholder;
use crate::protocol::protocol::Topic;

pub type MethodResult = (Result<Vec<u8>, Error>, Span);
pub type ComResult = (Result<HandleComIdResponse, Error>, Span);

pub struct SendMethod {
    pub method: Vec<u8>,
    pub channel: oneshot::Sender<MethodResult>,
    pub span: Span,
}

pub struct SendComIdRequest {
    pub request: HandleComIdRequest,
    pub channel: oneshot::Sender<ComResult>,
    pub span: Span,
}

pub struct AssemblePacket;

pub struct AbortSession;

pub struct RemoveSession {
    pub tsn: u32,
    pub hsn: u32,
}

pub struct SendPacket {
    pub source: Topic,
    pub packet: Packet,
    pub methods: Vec<(oneshot::Sender<MethodResult>, Span, MethodCallPlaceholder)>,
}

pub struct PacketSent {
    pub status: Option<Error>,
    pub methods: Vec<(oneshot::Sender<MethodResult>, Span, MethodCallPlaceholder)>,
}

pub struct ComIdRequestSent {
    pub status: Option<Error>,
    pub channel: oneshot::Sender<ComResult>,
    pub span: Span,
}

pub struct ComIdResponseReceived {
    pub response: HandleComIdResponse,
}

pub struct PacketReceived {
    pub packet: Packet,
}
