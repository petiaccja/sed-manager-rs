use crate::rpc::Error as RPCError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    RPCError(RPCError),
    IncompatibleSSC,
    NoAvailableSSC,
    AlreadyOwned,
}

impl From<RPCError> for Error {
    fn from(value: RPCError) -> Self {
        Self::RPCError(value)
    }
}
