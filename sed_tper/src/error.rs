use sed_spec::methods::MethodStatus;
use sorbit::error::Error as SorbitError;

use sed_device::Error as DeviceError;
use sed_packet::token::Error as TokenError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    // Encoding-related.
    #[error("Tokenization failure: {}", .0)]
    TokenError(TokenError),
    #[error("Invalid ComID response received from device: {}", .0)]
    InvalidComIdResponse(SorbitError),
    #[error("Invalid ComPacket received from device: {}", .0)]
    InvalidComPacket(SorbitError),

    // Protocol-related.
    #[error("Security command failure: {}", .0)]
    SecurityCommandFailed(DeviceError),
    #[error("The RPC session has been aborted")]
    Aborted,
    #[error("The RPC session is closed")]
    Closed,
    #[error("The RPC message has timed out")]
    TimedOut,

    // Data-related.
    #[error("Method call exceeds packet size limits")]
    MethodTooLarge,
    #[error("Token exceeds communication size limits")]
    TokenTooLarge,
    #[error("Received another message when a method call was expected")]
    MethodCallExpected,
    #[error("Received another message when a method result was expected")]
    MethodResultExpected,
    #[error("Received another message when an end of session message was expected")]
    EndOfSessionExpected,
    #[error("The returned values are not of the requested type/format")]
    ResultTypeMismatch,
    #[error("The reveived ComID response refers to an unexpected ComID")]
    ComIdResponseComIdMismatch,
    #[error("The reveived ComID response contains results of a different request")]
    ComIdResponseCodeMismatch,

    // RPC related.
    #[error("Method call failed: {}", .0)]
    MethodCallFailed(MethodStatus),
    #[error("Stack reset failed")]
    StackResetFailed,

    // General
    #[error("Operation not supported by the TPer")]
    NotSupported,
    #[error("Operation not implemented by SEDManager")]
    NotImplemented,
    #[error("Unspecified error (cause could not be determined)")]
    Unspecified,
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
