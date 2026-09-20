//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

mod ata;
mod generic;
mod nvme;
mod scsi;

use std::path::Path;

use crate::Error;
use crate::Interface;
use crate::StorageDevice;

pub use ata::AtaDevice;
pub use generic::GenericDevice;
pub use nvme::NvmeDevice;
pub use scsi::ScsiDevice;

fn into_boxed<ConcreteDevice: StorageDevice + 'static>(device: ConcreteDevice) -> Box<dyn StorageDevice> {
    Box::from(device) as Box<dyn StorageDevice>
}

pub async fn open_storage_device(path: impl AsRef<Path>) -> Result<Box<dyn StorageDevice>, Error> {
    let generic_device = GenericDevice::open(path).await?;
    match generic_device.interface() {
        Interface::NVMe => NvmeDevice::from_generic(generic_device).await.map(|dev| into_boxed(dev)),
        Interface::SCSI => ScsiDevice::from_generic(generic_device).await.map(|dev| into_boxed(dev)),
        Interface::ATA => AtaDevice::from_generic(generic_device).await.map(|dev| into_boxed(dev)),
        Interface::SATA => AtaDevice::from_generic(generic_device).await.map(|dev| into_boxed(dev)), // SATA is "same" as ATA.
        _ => Ok(into_boxed(generic_device)),
    }
}
