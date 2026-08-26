use std::sync::Arc;

use sed_manager::{Error, LockingConfigSession, SetupSession};
use sed_packet::MaxBytes;
use sed_spec::objects::AuthorityRef;
use sed_tper::Tper;

#[derive(Debug, Default)]
pub enum Session {
    #[default]
    None,
    Setup(SetupSession),
    LockingConfig(LockingConfigSession),
}

impl Session {
    /// Close the current session, returning to the [`None`] state.
    ///
    /// # Errors
    ///
    /// Closing a persistent session may fail. This can happen, for example,
    /// when the session has already been aborted by the device and no longer
    /// exists, or when there was a genuine communication error. In that case,
    /// it's recommended to perform a stack reset.
    pub async fn close(&mut self) -> Result<(), Error> {
        match core::mem::replace(self, Session::None) {
            Self::None => Ok(()),
            Self::Setup(_setup_session) => Ok(()), // Not a persistent session, just drop.
            Self::LockingConfig(locking_session) => locking_session.close().await,
        }
    }

    /// Start a setup session on the primary SSC of the TPer.
    ///
    /// # Errors
    ///
    /// The session is first [`close`]d. This might fail, in which case an error
    /// is returned. Opening the setup session might also fail.
    ///
    /// [`close`]: Self.close
    pub async fn start_setup_session(&mut self, tper: Arc<Tper>) -> Result<&SetupSession, Error> {
        // We could better inform the caller that the closing of the previous
        // session failed, but they should anyway just do a stack reset.
        self.close().await?;

        let sid_session = SetupSession::on_primary_ssc(tper).await?;
        *self = Self::Setup(sid_session);
        let Self::Setup(sid_session) = self else { unreachable!() };
        Ok(sid_session)
    }

    /// Start a locking config session on the primary SSC of the TPer.
    ///
    /// # Errors
    ///
    /// The session is first [`close`]d. This might fail, in which case an error
    /// is returned. Opening the locking config session might also fail.
    ///
    /// [`close`]: Self.close
    pub async fn start_locking_config_session(
        &mut self,
        tper: Arc<Tper>,
        authority: AuthorityRef,
        password: Option<MaxBytes<32>>,
    ) -> Result<&LockingConfigSession, Error> {
        self.close().await?;

        let locking_session = LockingConfigSession::login_on_primary_ssc(tper, authority, password).await?;
        *self = Self::LockingConfig(locking_session);
        let Self::LockingConfig(locking_session) = self else { unreachable!() };
        Ok(locking_session)
    }
}
