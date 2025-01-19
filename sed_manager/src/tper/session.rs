use crate::rpc::Error as RPCError;
use crate::rpc::SPSession;

pub struct Session {
    sp_session: SPSession,
}

impl Session {
    pub fn new(sp_session: SPSession) -> Self {
        Self { sp_session }
    }

    pub async fn close(&self) -> Result<(), RPCError> {
        match self.sp_session.call_eos().await {
            Ok(result) => {
                self.sp_session.close().await;
                Ok(result)
            }
            Err(err) => Err(err),
        }
    }
}
