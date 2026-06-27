use std::sync::Arc;

use sed_packet::MaxBytes;
use sed_spec::{
    methods::MethodStatus,
    objects::{AuthorityRef, CPinRefExt, SecurityProviderRefExt},
    types::LifeCycleState,
};
use sed_tper::Error as TperError;
use sed_tper::Tper;
use tracing::instrument;

use crate::{error::Error, spec::Spec};

/// Performs simple device setup and information retrieval operations.
///
/// The TPer sessions opened by this object are short-lived, and closed
/// immediately once they data has been read of written. From the perspective
/// of callers, this can still be regarded as a persistent session, as it will
/// anyways count towards the limit on the number of sessions the device
/// supports, though only intermittently.
#[derive(Debug)]
pub struct SetupSession {
    tper: Arc<Tper>,
    spec: Spec,
}

impl SetupSession {
    /// Open a session on the SSC given by `spec`.
    pub fn new(tper: Arc<Tper>, spec: Spec) -> Self {
        Self { tper, spec }
    }

    /// Open a session on the primary SSC of the TPer.
    ///
    /// # Errors
    ///
    /// An error is returned when no SSC or no recognzied SSC is found on the
    /// device.
    pub async fn on_primary_ssc(tper: Arc<Tper>) -> Result<Self, Error> {
        let discovery = tper.discover_current().await?;
        let spec = Spec::try_from(discovery).map_err(|_| Error::NoSscAvailable)?;
        Ok(Self::new(tper, spec))
    }

    pub fn spec(&self) -> &Spec {
        &self.spec
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
        let admin = &self.spec.admin;

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
        let admin = &self.spec.admin;
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
        let admin = &self.spec.admin;

        self.tper
            .start_session(admin.uid, Some(authority), Some(password))
            .await?
            .with(async |session| session.revert(admin.uid).await)
            .await?;

        Ok(())
    }

    /// Revert the secondary SP to its original factory state.
    ///
    /// This method requires a login to the Admin SP as SID.
    /// Only the secondary SP will be reverted, the Admin SP is unaffected.
    #[instrument(level = "info", skip(self, sid_password), ret, err)]
    pub async fn revert_secondary_sp(&self, sid_password: MaxBytes<32>) -> Result<(), Error> {
        let admin = &self.spec.admin;
        let secondary_sp_uid = self.spec.secondary_sp_uid().ok_or(Error::IncompatibleSsc)?;

        self.tper
            .start_session(admin.uid, Some(admin.authorities.sid), Some(sid_password))
            .await?
            .with(async |session| session.revert(secondary_sp_uid).await)
            .await?;

        Ok(())
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
        let secondary_sp_uid = self.spec.secondary_sp_uid().ok_or(Error::IncompatibleSsc)?;

        let session = self.tper.start_session(secondary_sp_uid, Some(admin), Some(password)).await?;
        match session.revert_sp(keep_global_range_key).await {
            Ok(()) => Ok(()),
            Err((session, err)) => {
                let _ = session.close().await;
                Err(err.into())
            }
        }
    }
}
