use nix::errno::Errno;

use crate::device;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("{}", .0)]
    Errno(Errno),
    #[error("Could not open /dev/disk/by-id to list devices")]
    NoDiskFolder,
}

impl From<Error> for device::Error {
    fn from(value: Error) -> Self {
        Self::PlatformError(value)
    }
}

impl From<Errno> for Error {
    fn from(value: Errno) -> Self {
        Self::Errno(value)
    }
}

impl From<Errno> for device::Error {
    fn from(value: Errno) -> Self {
        Self::PlatformError(Error::Errno(value))
    }
}
