//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use rustix::io::Errno;

use crate::Error as DeviceError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("{}", .0)]
    Errno(Errno),
    #[error("Could not open /dev/disk/by-id to list devices")]
    NoDiskFolder,
}

impl From<Errno> for Error {
    fn from(value: Errno) -> Self {
        Self::Errno(value)
    }
}

impl From<Errno> for DeviceError {
    fn from(value: Errno) -> Self {
        Self::PlatformError(Error::Errno(value))
    }
}
