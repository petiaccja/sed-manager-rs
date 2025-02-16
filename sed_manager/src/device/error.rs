use thiserror::Error;

#[cfg(target_os = "windows")]
use super::windows::Error as PlatformError;

#[cfg(target_os = "linux")]
use super::linux::Error as PlatformError;

#[derive(Debug, PartialEq, Eq, Clone, Error)]
pub enum Error {
    #[error("the provided data is too long")]
    DataTooLong,
    #[error("the provided buffer is too short to store requested data")]
    BufferTooShort,
    #[error("could not find the specified device")]
    DeviceNotFound,
    #[error("the provided arguments were invalid")]
    InvalidArgument,
    #[error("the provided protocol or com ID for IF-SEND/IF-RECV was invalid")]
    InvalidProtocolOrComID,
    #[error("the feature is not supported by the application")]
    NotImplemented,
    #[error("the feature is not supported by the device")]
    NotSupported,
    #[error("permission denied (are you root/admin?)")]
    PermissionDenied,
    #[error("an unspecified error occured (the source could not be determined)")]
    Unspecified,
    #[error("an operating-system-specific error occured {}", .0)]
    Source(PlatformError),
}
