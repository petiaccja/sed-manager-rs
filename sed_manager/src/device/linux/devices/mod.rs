use crate::device::{Device, Error};

pub fn open_device(_drive_path: &str) -> Result<Box<dyn Device>, Error> {
    Err(Error::NotImplemented)
}
