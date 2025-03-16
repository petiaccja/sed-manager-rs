use crate::device::{Device, Error};

mod ata;
mod nvme;

use ata::ATADevice;
use nvme::NVMeDevice;

fn replace_error(error: &mut Option<Error>, new_error: Error) {
    let is_only_mismatch = error.as_ref().is_some_and(|value| value == &Error::InterfaceNotSupported);
    if is_only_mismatch || error.is_none() {
        error.replace(new_error);
    }
}

pub fn open_device(drive_path: &str) -> Result<Box<dyn Device>, Error> {
    let mut error = Option::<Error>::None;

    match ATADevice::open(drive_path) {
        Ok(device) => return Ok(Box::new(device)),
        Err(new_error) => replace_error(&mut error, new_error),
    }
    match NVMeDevice::open(drive_path) {
        Ok(device) => return Ok(Box::new(device)),
        Err(new_error) => replace_error(&mut error, new_error),
    }

    Err(error.unwrap())
}
