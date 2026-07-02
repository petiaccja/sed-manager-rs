use sed_spec::methods::MethodStatus;
use sorbit::error::Error as SorbitError;

use sed_device::Error as DeviceError;
use sed_packet::{Uid, token::Error as TokenError};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    // Encoding-related.
    #[error("Tokenization failure: {}", .0)]
    TokenError(TokenError),
    #[error("Invalid ComID response received from device: {}", .0)]
    InvalidComIdResponse(SorbitError),
    #[error("Invalid ComPacket received from device: {}", .0)]
    InvalidComPacket(SorbitError),
    #[error("Invalid Discovery received from device: {}", .0)]
    InvalidDiscovery(SorbitError),

    // Protocol-related.
    #[error("Security command failed: {}", .0)]
    SecurityCommandFailed(DeviceError),
    #[error("The RPC session has been aborted")]
    Aborted,
    #[error("The RPC session is closed")]
    Closed,
    #[error("The RPC message has timed out")]
    TimedOut,
    #[error("Not allowed to send method `{0}` to the device")]
    MethodNotAllowed(Uid),

    // Data-related.
    #[error("Method ({requested} B) exceeds maximum method call size ({maximum} B)")]
    MethodTooLarge { requested: usize, maximum: usize },
    #[error("Received another message when an end of session message was expected")]
    EndOfSessionExpected,

    // RPC related.
    #[error("Method call failed: {}", .0)]
    MethodCallFailed(MethodStatus),
    #[error("The field `{object}.{field}` was not returned in the device's response")]
    FieldNotReturned { object: Uid, field: u16 },
    #[error("Stack reset failed")]
    StackResetFailed,

    // General
    #[error("Operation not supported by the TPer")]
    NotSupported,
    #[error("Operation not implemented by SEDManager")]
    NotImplemented,
}

impl From<DeviceError> for Error {
    fn from(value: DeviceError) -> Self {
        Self::SecurityCommandFailed(value)
    }
}

impl From<TokenError> for Error {
    fn from(value: TokenError) -> Self {
        Self::TokenError(value)
    }
}

impl From<MethodStatus> for Error {
    fn from(value: MethodStatus) -> Self {
        Self::MethodCallFailed(value)
    }
}
