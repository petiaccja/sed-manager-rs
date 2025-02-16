use tokio::sync::{oneshot, Mutex};

use crate::messaging::com_id::{
    ComIdState, HandleComIdRequest, HandleComIdResponse, StackResetResponsePayload, StackResetStatus,
    VerifyComIdValidResponsePayload,
};
use crate::rpc::{Error as RPCError, ErrorEvent as RPCErrorEvent, ErrorEventExt as _, MessageSender};
use crate::rpc::{Message, Tracked};
use crate::serialization::DeserializeBinary as _;

pub struct ComSession {
    sender: MessageSender,
    mutex: Mutex<()>,
}

impl ComSession {
    pub fn new(sender: MessageSender) -> Self {
        Self { sender, mutex: ().into() }
    }

    async fn do_request(&self, request: HandleComIdRequest) -> Result<HandleComIdResponse, RPCError> {
        let (tx, rx) = oneshot::channel();
        let _guard = self.mutex.lock().await;
        let _ = self.sender.send(Message::HandleComId { content: Tracked { item: request, promises: vec![tx] } });
        match rx.await {
            Ok(result) => result,
            Err(_) => Err(RPCErrorEvent::Closed.as_error()),
        }
    }
}

impl ComSession {
    pub async fn verify_com_id(&self, com_id: u16, com_id_ext: u16) -> Result<ComIdState, RPCError> {
        let request = HandleComIdRequest::verify_com_id_valid(com_id, com_id_ext);
        let response = self.do_request(request).await?;
        match VerifyComIdValidResponsePayload::from_bytes(response.payload.into_vec()) {
            Ok(response) => Ok(response.com_id_state),
            Err(err) => Err(err.as_error()),
        }
    }

    pub async fn stack_reset(&self, com_id: u16, com_id_ext: u16) -> Result<StackResetStatus, RPCError> {
        let request = HandleComIdRequest::stack_reset(com_id, com_id_ext);
        let response = self.do_request(request).await?;
        match StackResetResponsePayload::from_bytes(response.payload.into_vec()) {
            Ok(response) => Ok(response.stack_reset_status),
            Err(err) => Err(err.as_error()),
        }
    }
}
