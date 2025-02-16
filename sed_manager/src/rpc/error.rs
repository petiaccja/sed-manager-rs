use super::method::MethodStatus;
use crate::device::Error as DeviceError;
use crate::messaging::token::TokenizeError;
use crate::serialization::Error as SerializeError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("tokenization failed: {}", .0)]
    TokenizationFailed(TokenizeError),
    #[error("serialization failed: {}", .0)]
    SerializationFailed(SerializeError),
    #[error("IF-SEND failed: {}", .0)]
    SecuritySendFailed(DeviceError),
    #[error("IF-RECV failed: {}", .0)]
    SecurityReceiveFailed(DeviceError),
    #[error("operation/session has been aborted by the host")]
    AbortedByHost,
    #[error("operation/session has been aborted by the remote (TPer)")]
    AbortedByRemote,
    #[error("session is already closed")]
    Closed,
    #[error("timed out")]
    TimedOut,
    #[error("no response")]
    NoResponse,
    #[error("missing packet")]
    MissingPacket,
    #[error("invalid token stream encountered")]
    InvalidTokenStream,
    #[error("invalid credit control packet encountered")]
    InvalidCreditControl,
    #[error("invalid column type for specified table")]
    InvalidColumnType,
    #[error("no credits available to send the packet")]
    OutOfCreditRemote,
    #[error("serialized method's size exceeds limits")]
    MethodTooLarge,
    #[error("received another message when a method call was expected")]
    MethodCallExpected,
    #[error("received another message when a method result was expected")]
    MethodResultExpected,
    #[error("received another message when an end of session message was expected")]
    EOSExpected,
    #[error("method call failed: {}", .0)]
    MethodFailed(MethodStatus),
    #[error("operation not supported")]
    Unsupported,
    #[error("unspecified error occured")]
    Unspecified,
}

impl From<MethodStatus> for Error {
    fn from(value: MethodStatus) -> Self {
        Self::MethodFailed(value)
    }
}
