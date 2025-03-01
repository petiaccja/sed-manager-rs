use crate::messaging::com_id::{
    ComIdState, HandleComIdRequest, HandleComIdResponse, StackResetResponsePayload, StackResetStatus,
    VerifyComIdValidResponsePayload,
};
use crate::rpc::{CommandSender, Error as RPCError, ErrorEventExt as _};
use crate::serialization::DeserializeBinary as _;

pub struct ComSession {
    sender: CommandSender,
}

impl ComSession {
    pub fn new(sender: CommandSender) -> Self {
        Self { sender }
    }

    async fn do_request(&self, request: HandleComIdRequest) -> Result<HandleComIdResponse, RPCError> {
        self.sender.com_id(request).await
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

impl Drop for ComSession {
    fn drop(&mut self) {
        self.sender.close_com_session();
    }
}
