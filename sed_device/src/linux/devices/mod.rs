//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::path::Path;

use crate::{Device, Error};

mod ata;
mod nvme;

use ata::AtaDevice;
use nvme::NvmeDevice;

fn replace_error(error: &mut Option<Error>, new_error: Error) {
    let is_only_mismatch = error.as_ref().is_some_and(|value| value == &Error::InterfaceNotSupported);
    if is_only_mismatch || error.is_none() {
        error.replace(new_error);
    }
}

pub async fn open_device(path: impl AsRef<Path>) -> Result<Box<dyn Device>, Error> {
    let path = path.as_ref();
    let mut error = Option::<Error>::None;

    match AtaDevice::open(path).await {
        Ok(device) => return Ok(Box::new(device)),
        Err(new_error) => replace_error(&mut error, new_error),
    }
    match NvmeDevice::open(path).await {
        Ok(device) => return Ok(Box::new(device)),
        Err(new_error) => replace_error(&mut error, new_error),
    }

    Err(error.unwrap())
}
