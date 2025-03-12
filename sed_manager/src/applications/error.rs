use crate::rpc::Error as RPCError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("{}", .0)]
    RPCError(RPCError),
    #[error("Feature not supported by the TCG Security Subsystem Class")]
    IncompatibleSSC,
    #[error("The device does not support any TCG Security Subsystem Classes")]
    NoAvailableSSC,
    #[error("Ownership has already been set up on this device")]
    AlreadyOwned,
    #[error("Locking has already been activated on this device")]
    AlreadyActivated,
    #[error("Internal error: this is a bug in SEDManager")]
    InternalError,
    #[error("Cancelled")]
    Cancelled,
    #[error("Could not open file")]
    FileNotOpen,
    #[error("Could not read file")]
    FileReadError,
    #[error("The file is too large")]
    FileTooLarge,
    #[error("Invalid ACE expression")]
    InvalidACEExpression,
}

impl From<RPCError> for Error {
    fn from(value: RPCError) -> Self {
        Self::RPCError(value)
    }
}
