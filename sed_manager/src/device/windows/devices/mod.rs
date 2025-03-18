mod ata;
mod generic;
mod nvme;
mod scsi;

use crate::device::Device;
use crate::device::Error;
use crate::device::Interface;

pub use ata::ATADevice;
pub use generic::GenericDevice;
pub use nvme::NVMeDevice;
pub use scsi::SCSIDevice;

fn into_boxed<ConcreteDevice: Device + 'static>(device: ConcreteDevice) -> Box<dyn Device> {
    Box::from(device) as Box<dyn Device>
}

pub fn open_device(drive_path: &str) -> Result<Box<dyn Device>, Error> {
    let generic_device = GenericDevice::open(drive_path)?;
    match generic_device.interface() {
        Interface::NVMe => NVMeDevice::try_from(generic_device).map(|dev| into_boxed(dev)),
        Interface::SCSI => SCSIDevice::try_from(generic_device).map(|dev| into_boxed(dev)),
        Interface::ATA => ATADevice::try_from(generic_device).map(|dev| into_boxed(dev)),
        Interface::SATA => ATADevice::try_from(generic_device).map(|dev| into_boxed(dev)), // SATA is "same" as ATA.
        _ => Ok(into_boxed(generic_device)),
    }
}
