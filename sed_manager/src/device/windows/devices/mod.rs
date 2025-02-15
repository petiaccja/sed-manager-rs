mod generic;
mod nvme;
mod scsi;

use crate::device::Device;
use crate::device::Error;

pub use generic::GenericDevice;
pub use nvme::NVMeDevice;

pub fn open_device(drive_path: &str) -> Result<Box<dyn Device>, Error> {
    let generic_device = GenericDevice::open(drive_path)?;
    let generic_device = match NVMeDevice::try_from(generic_device) {
        Ok(nvme_device) => return Ok(Box::from(nvme_device)),
        Err(generic_device) => generic_device,
    };
    Ok(Box::from(generic_device))
}
