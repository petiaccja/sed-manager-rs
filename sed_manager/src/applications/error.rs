use crate::rpc::Error as RPCError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("{}", .0)]
    RPCError(RPCError),
    #[error("the security subsystem class does not support the operation")]
    IncompatibleSSC,
    #[error("the device does not support any security subsystem classes")]
    NoAvailableSSC,
    #[error("someone has already taken ownership of this device")]
    AlreadyOwned,
    #[error("someone has already activated locking on this device")]
    AlreadyActivated,
    #[error("an internal error occured: this is a bug")]
    InternalError,
    #[error("the task was cancelled")]
    Cancelled,
    #[error("could not open the file")]
    FileNotOpen,
    #[error("could not read the file")]
    FileReadError,
}

impl From<RPCError> for Error {
    fn from(value: RPCError) -> Self {
        Self::RPCError(value)
    }
}
