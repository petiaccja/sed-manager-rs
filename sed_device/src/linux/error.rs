//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use rustix::io::Errno;

use crate::Error as DeviceError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct Error(Errno);

impl From<Errno> for Error {
    fn from(value: Errno) -> Self {
        Self(value)
    }
}

impl From<Errno> for DeviceError {
    fn from(value: Errno) -> Self {
        Self::PlatformError(Error(value))
    }
}
