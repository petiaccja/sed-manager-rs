use super::method::MethodStatus;
use crate::device::Error as DeviceError;
use crate::messaging::token::TokenStreamError;
use crate::serialization::Error as SerializeError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    // Encoding-related.
    #[error("tokenization failed: {}", .0)]
    TokenStreamFailed(TokenStreamError),
    #[error("serialization failed: {}", .0)]
    SerializationFailed(SerializeError),

    // Protocol-related.
    #[error("security command failed: {}", .0)]
    SecurityCommandFailed(DeviceError),
    #[error("operation/session has been aborted by the host")]
    Aborted,
    #[error("operation/session has been aborted by the remote (TPer)")]
    Closed,
    #[error("timed out")]
    TimedOut,

    // Data-related.
    #[error("invalid column type for specified table")]
    InvalidColumnType,
    #[error("serialized method's size exceeds limits")]
    MethodTooLarge,
    #[error("received another message when a method call was expected")]
    MethodCallExpected,
    #[error("received another message when a method result was expected")]
    MethodResultExpected,
    #[error("received another message when an end of session message was expected")]
    EOSExpected,

    // Method-related.
    #[error("method call failed: {}", .0)]
    MethodFailed(MethodStatus),

    // General
    #[error("operation not supported by the TPer")]
    NotSupported,
    #[error("operation not implemented by the application")]
    NotImplemented,
    #[error("unspecified error occured")]
    Unspecified,
}

impl From<TokenStreamError> for Error {
    fn from(value: TokenStreamError) -> Self {
        Error::TokenStreamFailed(value)
    }
}

impl From<SerializeError> for Error {
    fn from(value: SerializeError) -> Self {
        Error::SerializationFailed(value)
    }
}

impl From<DeviceError> for Error {
    fn from(value: DeviceError) -> Self {
        Error::SecurityCommandFailed(value)
    }
}

impl From<MethodStatus> for Error {
    fn from(value: MethodStatus) -> Self {
        Error::MethodFailed(value)
    }
}
