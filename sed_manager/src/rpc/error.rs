use super::method::MethodStatus;
use crate::device::Error as DeviceError;
use crate::messaging::token::TokenStreamError;
use crate::serialization::Error as SerializeError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErrorEvent {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorAction {
    Unspecified,
    Send,
    Receive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub event: ErrorEvent,
    pub action: ErrorAction,
}

pub trait ErrorEventExt: Into<ErrorEvent> {
    fn while_sending(self) -> Error {
        Error { event: self.into(), action: ErrorAction::Send }
    }
    fn while_receiving(self) -> Error {
        Error { event: self.into(), action: ErrorAction::Receive }
    }
    fn as_error(self) -> Error {
        Error { event: self.into(), action: ErrorAction::Unspecified }
    }
}

impl From<TokenStreamError> for ErrorEvent {
    fn from(value: TokenStreamError) -> Self {
        Self::TokenStreamFailed(value)
    }
}

impl From<SerializeError> for ErrorEvent {
    fn from(value: SerializeError) -> Self {
        Self::SerializationFailed(value)
    }
}

impl From<DeviceError> for ErrorEvent {
    fn from(value: DeviceError) -> Self {
        Self::SecurityCommandFailed(value)
    }
}

impl From<MethodStatus> for ErrorEvent {
    fn from(value: MethodStatus) -> Self {
        Self::MethodFailed(value)
    }
}

impl From<ErrorEvent> for Error {
    fn from(value: ErrorEvent) -> Self {
        value.as_error()
    }
}

impl<T: Into<ErrorEvent>> ErrorEventExt for T {}

impl Error {
    pub fn while_sending(self) -> Self {
        Self { event: self.event, action: ErrorAction::Send }
    }
    pub fn while_receiving(self) -> Self {
        Self { event: self.event, action: ErrorAction::Receive }
    }
    pub fn as_error(self) -> Self {
        self
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.action {
            ErrorAction::Unspecified => write!(f, "{}", self.event),
            ErrorAction::Send => write!(f, "[sending] {}", self.event),
            ErrorAction::Receive => write!(f, "[receiving] {}", self.event),
        }
    }
}

impl std::error::Error for Error {}
