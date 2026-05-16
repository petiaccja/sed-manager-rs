use std::sync::Arc;

use sed_packet::MaxBytes;
use sed_spec::{
    methods::MethodStatus,
    objects::{AuthorityRef, CPinRefExt, SecurityProviderRefExt},
    types::LifeCycleState,
};
use sed_tper::Tper;
use sed_tper::error::Error as TperError;
use tracing::instrument;

use crate::{error::Error, spec::Spec};

pub struct SidSession {
    tper: Arc<Tper>,
    spec: Spec,
}

impl SidSession {
    pub fn new(tper: Arc<Tper>, spec: Spec) -> Self {
        Self { tper, spec }
    }

    pub async fn on_primary_ssc(tper: Arc<Tper>) -> Result<Self, Error> {
        let discovery = tper.discover_now().await?;
        let spec = Spec::new(discovery);
        Ok(Self::new(tper, spec))
    }

    /// Take ownership of the device by changing the SID password.
    ///
    /// # Errors
    ///
    /// To change the SID password, we first have to authenticate the SID
    /// authority using its initial (MSID) password. If ownership has already
    /// been taken, the MSID password won't work, and an error is returned.
    ///
    /// The method may also fail due to any common Tper RPC errors.
    #[instrument(level = "info", skip(self, new_sid_password), ret, err)]
    pub async fn take_owneship(&self, new_sid_password: MaxBytes<32>) -> Result<(), Error> {
        let admin = self.spec.admin.as_ref().ok_or(Error::NoSscSupported)?;

        // Get the MSID password.
        let initial_password = self
            .tper
            .start_session(admin.uid, None, None)
            .await?
            .with(async |session| session.get_field(admin.c_pins.msid.pin()).await)
            .await?;

        // Change the SID password to the new one.
        self.tper
            .start_session(admin.uid, Some(admin.authorities.sid), Some(initial_password))
            .await
            .map_err(|err| match err {
                TperError::MethodCallFailed(MethodStatus::NotAuthorized) => Error::AlreadyOwned,
                err => err.into(),
            })?
            .with(async |session| session.set_field(admin.c_pins.sid.pin(), new_sid_password).await)
            .await?;

        Ok(())
    }

    /// Activate the locking or key per I/O SPs.
    ///
    /// # Errors
    ///
    /// The SID authority is used to activate the locking or KPIO SP. If the SP
    /// is already in the [`Manufactured`] state (i.e. already activated),
    /// [`Error::AlreadyActivated`] is returned.
    ///
    /// The method may also fail due to any common Tper RPC errors.
    ///
    /// [`Manufactured`]: LifeCycleState::Manufactured
    #[instrument(level = "info", skip(self, sid_password), ret, err)]
    pub async fn activate_secondary_sp(&self, sid_password: MaxBytes<32>) -> Result<(), Error> {
        let admin = self.spec.admin.as_ref().ok_or(Error::NoSscSupported)?;
        let secondary_sp_uid = self
            .spec
            .locking
            .as_ref()
            .map(|sp| sp.uid)
            .or_else(|| self.spec.kpio.as_ref().map(|sp| sp.uid))
            .ok_or(Error::IncompatibleSsc)?;

        self.tper
            .start_session(admin.uid, Some(admin.authorities.sid), Some(sid_password))
            .await?
            .with(async |session| {
                let life_cycle_state = session.get_field(secondary_sp_uid.life_cycle_state()).await?;
                if life_cycle_state != LifeCycleState::ManufacturedInactive {
                    return Err(Error::AlreadyActivated);
                }
                session.activate(secondary_sp_uid).await?;
                Ok(())
            })
            .await?;

        Ok(())
    }

    /// Revert the whole device to its original factory state.
    ///
    /// This method requires a login to the Admin SP as SID or PSID. Both the
    /// admin and the secondary (locking or KPIO) SP will be reverted to their
    /// original factory state.
    ///
    /// # Parameters
    ///
    /// - `authority`: this may be the SID or the PSID authority.
    /// - `password`: the password of the `authority`.
    #[instrument(level = "info", skip(self, password), ret, err)]
    pub async fn revert_tper(&self, authority: AuthorityRef, password: MaxBytes<32>) -> Result<(), Error> {
        todo!()
    }

    /// Revert the secondary SP to its original factory state.
    ///
    /// This method requires a login to the Admin SP as SID.
    /// Only the secondary SP will be reverted, the Admin SP is unaffected.
    #[instrument(level = "info", skip(self, sid_password), ret, err)]
    pub async fn revert_secondary_sp(&self, sid_password: MaxBytes<32>) -> Result<(), Error> {
        todo!()
    }

    /// Revert the secondary SP to its original factory state.
    ///
    /// This method requires a login to the secondary SP as one of the admins.
    /// Only the secondary SP will be reverted, the Admin SP is unaffected.
    #[instrument(level = "info", skip(self, password), ret, err)]
    pub async fn revert_secondary_sp_ex(
        &self,
        admin: AuthorityRef,
        password: MaxBytes<32>,
        keep_global_range_key: Option<bool>,
    ) -> Result<(), Error> {
        todo!()
    }
}
