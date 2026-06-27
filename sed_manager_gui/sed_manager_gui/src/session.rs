use std::sync::Arc;

use sed_manager::{Error, SetupSession};
use sed_tper::Tper;

#[derive(Debug, Default)]
pub enum Session {
    #[default]
    None,
    Setup(SetupSession),
}

impl Session {
    pub async fn close(&mut self) {
        match core::mem::replace(self, Session::None) {
            Session::None => (),
            Session::Setup(_sid_session) => (), // Not a persistent session, just drop.
        }
    }

    pub async fn start_sid_session(&mut self, tper: Arc<Tper>) -> Result<&SetupSession, Error> {
        match self {
            Self::None => {
                let sid_session = SetupSession::on_primary_ssc(tper).await?;
                *self = Self::Setup(sid_session);
                let Self::Setup(sid_session) = self else { unreachable!() };
                Ok(sid_session)
            }
            Self::Setup(sid_session) => Ok(sid_session),
        }
    }
}
