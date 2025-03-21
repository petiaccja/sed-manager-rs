use sed_manager::applications::Error as AppError;
use sed_manager::device::Error as DeviceError;
use sed_manager::rpc::Error as RPCError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("Exit requested")]
    Quit,
    #[error("No locked devices found")]
    NoDevice,
    #[error("Username invalid")]
    InvalidUser,
    #[error("{}", .0)]
    DeviceError(DeviceError),
    #[error("{}", .0)]
    RPCError(RPCError),
    #[error("{}", .0)]
    AppError(AppError),
}

impl From<DeviceError> for Error {
    fn from(value: DeviceError) -> Self {
        Self::DeviceError(value)
    }
}

impl From<RPCError> for Error {
    fn from(value: RPCError) -> Self {
        Self::RPCError(value)
    }
}

impl From<AppError> for Error {
    fn from(value: AppError) -> Self {
        Self::AppError(value)
    }
}
