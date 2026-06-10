use std::sync::Arc;

use sed_manager::{Error, SidSession};
use sed_tper::Tper;

#[derive(Debug, Default)]
pub enum Session {
    #[default]
    None,
    Sid(SidSession),
}

impl Session {
    pub async fn close(&mut self) {
        match core::mem::replace(self, Session::None) {
            Session::None => (),
            Session::Sid(_sid_session) => (), // Not a persistent session, just drop.
        }
    }

    pub async fn start_sid_session(&mut self, tper: Arc<Tper>) -> Result<&SidSession, Error> {
        match self {
            Self::None => {
                let sid_session = SidSession::on_primary_ssc(tper).await?;
                *self = Self::Sid(sid_session);
                let Self::Sid(sid_session) = self else { unreachable!() };
                Ok(sid_session)
            }
            Self::Sid(sid_session) => Ok(sid_session),
        }
    }
}
