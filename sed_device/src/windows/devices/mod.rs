//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

mod ata;
mod generic;
mod nvme;
mod raw_device;
mod scsi;

use std::path::Path;

use crate::Device;
use crate::Error;
use crate::Interface;

pub use ata::AtaDevice;
pub use generic::GenericDevice;
pub use nvme::NvmeDevice;
pub use scsi::ScsiDevice;

fn into_boxed<ConcreteDevice: Device + 'static>(device: ConcreteDevice) -> Box<dyn Device> {
    Box::from(device) as Box<dyn Device>
}

pub async fn open_device(path: impl AsRef<Path>) -> Result<Box<dyn Device>, Error> {
    let generic_device = GenericDevice::open(path).await?;
    match generic_device.interface() {
        Interface::NVMe => NvmeDevice::from_generic(generic_device).await.map(|dev| into_boxed(dev)),
        Interface::SCSI => ScsiDevice::try_from(generic_device).map(|dev| into_boxed(dev)),
        Interface::ATA => AtaDevice::from_generic(generic_device).await.map(|dev| into_boxed(dev)),
        Interface::SATA => AtaDevice::from_generic(generic_device).await.map(|dev| into_boxed(dev)), // SATA is "same" as ATA.
        _ => Ok(into_boxed(generic_device)),
    }
}
