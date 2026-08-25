use std::sync::Arc;

use sed_packet::MaxBytes;
use sed_spec::objects::AuthorityRef;
use sed_tper::{Session, Tper};
use tracing::instrument;

use crate::{error::Error, spec::Spec};

/// Configures the locking SP of the TPer, like locking ranges and authorities.
///
/// This is a persistent session and holds a [`Session`] object for the
/// lifetime of the object. Requests to configure the locking SP are performed
/// within that session.
#[derive(Debug)]
pub struct LockingConfigSession {
    spec: Spec,
    session: Session,
}

impl LockingConfigSession {
    /// Start the configuration session on the locking SP as `authority`. The
    /// permissions of the authority determine which actions can be performed.
    ///
    /// For devices that support multiple SSCs, the chosen SSC is identified
    /// by the `spec`.
    #[instrument(level = "info", skip(tper, password), ret, err)]
    pub async fn login(
        tper: Arc<Tper>,
        spec: Spec,
        authority: AuthorityRef,
        password: Option<MaxBytes<32>>,
    ) -> Result<Self, Error> {
        let locking_sp_uid = spec.locking.as_ref().map(|sp| sp.uid).ok_or(Error::IncompatibleSsc)?;
        let session = tper.start_session(locking_sp_uid, Some(authority), password).await?;
        Ok(Self { spec, session })
    }

    /// Start the configuration session on the locking SP as `authority`. The
    /// permissions of the authority determine which actions can be performed.
    ///
    /// For devices that support multiple SSCs, the primary SSC is chosen. See
    /// [`Spec`] about how the primary SSC is chosen.
    #[instrument(level = "info", skip(tper, password), ret, err)]
    pub async fn login_on_primary_ssc(
        tper: Arc<Tper>,
        authority: AuthorityRef,
        password: Option<MaxBytes<32>>,
    ) -> Result<Self, Error> {
        let discovery = tper.discover_current().await?;
        let spec = Spec::try_from(discovery).map_err(|_| Error::NoSscAvailable)?;
        Self::login(tper, spec, authority, password).await
    }

    pub fn spec(&self) -> &Spec {
        &self.spec
    }

    /// Close the internal session to the SP. See [`Session::close`] for how
    /// [`Drop`] is handled.
    pub async fn close(self) -> Result<(), Error> {
        self.session.close().await.map_err(|err| err.into())
    }
}
