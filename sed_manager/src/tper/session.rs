use crate::async_finalize::async_finalize;
use crate::async_finalize::sync_finalize;
use crate::async_finalize::AsyncFinalize;
use crate::messaging::types::AuthorityUID;
use crate::messaging::types::BoolOrBytes;
use crate::messaging::value::Bytes;
use crate::rpc::args::DecodeArgs;
use crate::rpc::args::EncodeArgs;
use crate::rpc::Error as RPCError;
use crate::rpc::MethodCall;
use crate::rpc::MethodStatus;
use crate::rpc::SPSession;
use crate::specification::invokers;
use crate::specification::methods;

pub struct Session {
    sp_session: SPSession,
}

impl Session {
    pub fn new(sp_session: SPSession) -> Self {
        Self { sp_session }
    }

    pub async fn end_session(&mut self) -> Result<(), RPCError> {
        let result = self.sp_session.end().await;
        result
    }

    pub async fn authenticate(&self, authority: AuthorityUID, proof: Option<Bytes>) -> Result<BoolOrBytes, RPCError> {
        let call = MethodCall {
            invoking_id: invokers::THIS_SP,
            method_id: methods::AUTHENTICATE,
            args: (authority, proof).encode_args(),
            status: MethodStatus::Success,
        };
        let result = self.sp_session.call(call).await?;
        if result.status != MethodStatus::Success {
            return Err(RPCError::MethodFailed(result.status));
        }
        let (success,): (BoolOrBytes,) = result.results.decode_args()?;
        Ok(success)
    }

    pub async fn get(&self) {
        todo!()
    }

    pub async fn set(&self) {
        todo!()
    }

    pub async fn next(&self) {
        todo!()
    }

    pub async fn get_acl(&self) {
        todo!()
    }

    pub async fn gen_key(&self) {
        todo!()
    }

    pub async fn revert(&self) {
        todo!()
    }

    pub async fn revert_sp(&self) {
        todo!()
    }

    pub async fn activate(&self) {
        todo!()
    }

    pub async fn random(&self) {
        todo!()
    }
}

impl AsyncFinalize for Session {
    async fn finalize(&mut self) {
        let _ = self.end_session().await;
        async_finalize(&mut self.sp_session).await;
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        sync_finalize(self);
    }
}
