//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use super::method::MethodStatus;
use crate::device::Error as DeviceError;
use crate::messaging::token::TokenStreamError;
use crate::serialization::Error as SerializeError;

use sorbit::error::Error as SorbitError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    // Encoding-related.
    #[error("Tokenization failure: {}", .0)]
    TokenStreamFailed(TokenStreamError),
    #[error("Serialization failure: {}", .0)]
    SerializationFailed(SerializeError),
    #[error("Serialization failure (sorbit): {}", .0)]
    SerializationFailedSorbit(SorbitError),

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
    EOSExpected,
    #[error("The returned values are not of the requested type/format")]
    ResultTypeMismatch,
    #[error("The reveived ComID response refers to an unexpected ComID")]
    ComIDResponseComIDMismatch,
    #[error("The reveived ComID response contains results of a different request")]
    ComIDResponseCodeMismatch,

    // Method-related.
    #[error("{}", .0)]
    MethodFailed(MethodStatus),

    // General
    #[error("Operation not supported by the TPer")]
    NotSupported,
    #[error("Operation not implemented by SEDManager")]
    NotImplemented,
    #[error("Unspecified error (cause could not be determined)")]
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

impl From<SorbitError> for Error {
    fn from(value: SorbitError) -> Self {
        Error::SerializationFailedSorbit(value)
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
