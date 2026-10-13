//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_device::Error as DeviceError;
use sed_tper::Error as TperError;

use crate::locking_range_plan::LockingRangePlanError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("{}", .0)]
    DeviceError(#[from] DeviceError),
    #[error("{}", .0)]
    TperError(#[from] TperError),
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
    #[error("The desired locking ranges won't be applied: {0}")]
    LockingRangeUpdateRejected(#[from] LockingRangePlanError),
    #[error(
        "An error occurred while applying the desired locking ranges: {0}. The updates may have been partially applied."
    )]
    LockingRangeUpdateFailed(#[source] TperError),
}
