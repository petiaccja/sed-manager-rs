//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_device::Error as DeviceError;
use sed_tper::Error as TperError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("{}", .0)]
    DeviceError(DeviceError),
    #[error("{}", .0)]
    TperError(TperError),
    #[error("The feature is not supported by the current TCG SSC")]
    IncompatibleSsc,
    #[error("The device does not support any TCG SSCs")]
    NoSscAvailable,
    #[error("Ownership has already been set up on this device")]
    AlreadyOwned,
    #[error("Locking has already been activated on this device")]
    AlreadyActivated,
    #[error("Internal error: this is a bug in SEDManager")]
    InternalError,
    #[error("Cancelled")]
    Cancelled,
    #[error("Cannot open file")]
    FileNotOpen,
    #[error("Cannot read file")]
    FileReadError,
    #[error("File is too large")]
    FileTooLarge,
    #[error("Invalid ACE expression")]
    InvalidAceExpression,
    #[error("Not a password authority")]
    NotAPasswordAuthority,
}

impl From<TperError> for Error {
    fn from(value: TperError) -> Self {
        Self::TperError(value)
    }
}

impl From<DeviceError> for Error {
    fn from(value: DeviceError) -> Self {
        Self::DeviceError(value)
    }
}
