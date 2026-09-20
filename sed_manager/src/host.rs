use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use sed_async::PolyRuntime;
use sed_device::{list_storage_devices, open_storage_device};
use tracing::instrument;

use crate::{Device, Error};

/// The entry point to access the storage devices of the host computer system.
///
/// You might wonder why we need a `struct` for this. To separate the UI and the
/// backend into separate processes for security, this would be turned into an
/// RPC service, requiring a `struct`. This is just prep work here.
#[derive(Debug)]
pub struct Host {
    runtime: Arc<PolyRuntime>,
}

impl Host {
    /// List the physical drives present in the system.
    /// See [`list_storage_devices`] to learn more.
    #[instrument(level = "info", skip(self), ret, err)]
    pub async fn list_devices(&self) -> Result<Vec<PathBuf>, Error> {
        #[allow(unused_mut)]
        let mut devices = list_storage_devices().await?;
        #[cfg(any(feature = "virtual_device"))]
        devices.push(sed_virtual_device::VIRTUAL_DEVICE_PATH.into());
        Ok(devices)
    }

    /// Opens a drive by the path identifying it.
    /// See [`open_storage_device`] to learn more.
    #[instrument(level = "info", skip(self, path), err)]
    pub async fn open_device(&self, path: impl AsRef<Path>) -> Result<Device, Error> {
        #[cfg(any(feature = "virtual_device"))]
        if path.as_ref() == sed_virtual_device::VIRTUAL_DEVICE_PATH {
            use sed_virtual_device::VirtualDevice;

            return Ok(Device::new(Box::new(VirtualDevice::new()), self.runtime.clone()).await);
        }
        let device = open_storage_device(path).await?;
        Ok(Device::new(device, self.runtime.clone()).await)
    }
}
