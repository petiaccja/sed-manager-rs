use std::sync::Arc;

use sed_async::PolyRuntime;
use sed_device::StorageDevice;
use sed_packet::{MaxBytes, com_id::ComIdState, discovery::Discovery};
use sed_spec::objects::AuthorityRef;
use sed_tper::{PropertiesChanged, Tper};
use tracing::instrument;

use crate::{Error, LockingConfigSession, SetupSession, Spec};

#[derive(Debug)]
pub struct Device {
    device: Arc<dyn StorageDevice>,
    spec: Result<Spec, Error>,
    tper: Option<Tper>,
}

impl Device {
    /// Create a new device holding a reference the to provided [`StorageDevice`].
    #[instrument(level = "info")]
    pub async fn new(device: Box<dyn StorageDevice>, runtime: Arc<PolyRuntime>) -> Self {
        let device = Arc::from(device);
        let spec = Tper::discover(&*device)
            .await
            .map_err(Error::from)
            .map(|discovery| Spec::try_from(discovery).map_err(|_| Error::NoSscAvailable))
            .flatten();
        let tper = spec
            .as_ref()
            .ok()
            .and_then(|spec| spec.default_ssc())
            .and_then(|ssc| ssc.as_ssc())
            .map(|ssc| Tper::connect(ssc.static_com_ids_p1().start, 0, device.clone(), runtime));
        Self { device, spec, tper }
    }

    /// Discover the SED capabilities of the device.
    /// For more information, see [`Tper::discover`].
    #[instrument(level = "info", skip(self), ret, err)]
    pub async fn discover(&self) -> Result<Discovery, Error> {
        Tper::discover(&*self.device).await.map_err(|err| err.into())
    }

    /// Query the status of the ComID on which the device is connected.
    #[instrument(level = "info", skip(self), ret, err)]
    pub async fn query_com_id_state(&self) -> Result<ComIdState, Error> {
        let tper = self.tper.as_ref().ok_or(Error::NoSscAvailable)?;
        tper.verify_com_id_valid(tper.com_id(), tper.com_id_ext()).await.map_err(Into::into)
    }

    /// Reset the device's protocol stack.
    ///
    /// This terminates all currently running sessions on the device.
    #[instrument(level = "info", skip(self), ret, err)]
    pub async fn stack_reset(&self) -> Result<(), Error> {
        let tper = self.tper.as_ref().ok_or(Error::NoSscAvailable)?;
        tper.stack_reset(tper.com_id(), tper.com_id_ext()).await.map_err(Into::into)
    }

    /// Start a setup session on the device's Tper.
    /// See [`SetupSession`].
    #[instrument(level = "info", skip(self), ret, err)]
    pub async fn start_setup_session(&self) -> Result<SetupSession, Error> {
        let spec = self.spec.clone()?;
        let tper = self.tper.as_ref().ok_or(Error::NoSscAvailable)?;
        Ok(SetupSession::new(tper, spec))
    }

    /// Start a locking configuration session on the device's Tper.
    /// See [`LockingConfigSession`].
    #[instrument(level = "info", skip(self), ret, err)]
    pub async fn start_locking_config_session(
        &self,
        authority: AuthorityRef,
        password: Option<MaxBytes<32>>,
    ) -> Result<LockingConfigSession, Error> {
        let spec = self.spec.clone()?;
        let tper = self.tper.as_ref().ok_or(Error::NoSscAvailable)?;
        LockingConfigSession::login(tper, spec, authority, password).await
    }

    /// Listen to changes in the connection properties.
    /// See [`Tper::properties_changed`].
    pub fn properties_changed(&self) -> Result<async_broadcast::Receiver<PropertiesChanged>, Error> {
        self.tper.as_ref().ok_or(Error::NoSscAvailable).map(|tper| tper.properties_changed())
    }

    /// Close the device. This closes the file handle to the device and
    /// terminates the TPer protocol stack as well. The function will only
    /// return once all session holding the protocol stack alive are closed.
    ///
    /// See [`Tper::close`] for lifecyle information.
    pub async fn close(self) {
        if let Some(tper) = self.tper {
            let _ = tper.close().await;
        }
    }
}
