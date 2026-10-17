//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_device::Error as DeviceError;
use sed_tper::Error as TperError;

use crate::{authority_plan::AuthorityPlanError, locking_range_plan::LockingRangePlanError};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("{}", .0)]
    DeviceError(#[from] DeviceError),
    #[error("{}", .0)]
    TperError(#[from] TperError),
    #[error("The feature is not supported by the current TCG SSC")]
    IncompatibleSsc,
    #[error("The feature is not supported by the device")]
    NotSupported,
    #[error("The device does not support any TCG SSCs")]
    NoSscAvailable,
    #[error("The feature is supported by the device, but isn't not implemented")]
    NotImplemented,
    #[error("Ownership has already been set up on this device")]
    AlreadyOwned,
    #[error("Locking has already been activated on this device")]
    AlreadyActivated,
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
    #[error("The desired authorities won't be applied: {0}")]
    AuthorityUpdateRejected(#[from] AuthorityPlanError),
    #[error(
        "An error occurred while applying the desired authorities: {0}. The updates may have been partially applied."
    )]
    AuthorityUpdateFailed(#[source] TperError),
}
