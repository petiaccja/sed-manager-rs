#[cfg(target_os = "windows")]
use super::windows::Error as PlatformError;

#[cfg(target_os = "linux")]
use super::linux::Error as PlatformError;

#[derive(Debug, PartialEq, Eq, Clone, thiserror::Error)]
pub enum Error {
    #[error("Buffer too short to receive data")]
    BufferTooShort,
    #[error("Buffer too large and not supported")]
    BufferTooLarge,
    #[error("Buffer has invalid alignment")]
    InvalidAlignment,

    #[error("Could not find device")]
    DeviceNotFound,
    #[error("Invalid argument")]
    InvalidArgument,
    #[error("Invalid security protocol or ComID")]
    InvalidProtocolOrComID,
    #[error("Feature not supported by SEDManager")]
    NotImplemented,
    #[error("Feature not supported by the device")]
    NotSupported,
    #[error("Permission denied (retry with elevated privileges)")]
    PermissionDenied,
    #[error("Could not open a device using the explicitly selected interface")]
    InterfaceMismatch,

    #[error("The ATA command was aborted")]
    ATACommandAborted,
    #[error("The SCSI command failed")]
    SCSICommandFailed,

    #[error("Unspecified error occured (the exact cause could not be determined)")]
    Unspecified,

    #[error("{}", .0)]
    PlatformError(PlatformError),
}
