//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::messaging::com_id::{
    ComIdState, HandleComIdRequest, HandleComIdResponse, HandleComIdResponseParams, StackResetStatus,
};
use crate::rpc::{CommandSender, Error as RPCError};

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
        if response.com_id != com_id || response.com_id_ext != com_id_ext {
            return Err(RPCError::ComIDResponseComIDMismatch);
        }
        if let HandleComIdResponseParams::VerifyComIdValid { com_id_state, .. } = response.params {
            Ok(com_id_state)
        } else {
            Err(RPCError::ComIDResponseCodeMismatch)
        }
    }

    pub async fn stack_reset(&self, com_id: u16, com_id_ext: u16) -> Result<StackResetStatus, RPCError> {
        let request = HandleComIdRequest::stack_reset(com_id, com_id_ext);
        let response = self.do_request(request).await?;
        if response.com_id != com_id || response.com_id_ext != com_id_ext {
            return Err(RPCError::ComIDResponseComIDMismatch);
        }
        if let HandleComIdResponseParams::StackReset { status, .. } = response.params {
            Ok(status)
        } else {
            Err(RPCError::ComIDResponseCodeMismatch)
        }
    }
}

impl Drop for ComSession {
    fn drop(&mut self) {
        self.sender.close_com_session();
    }
}
