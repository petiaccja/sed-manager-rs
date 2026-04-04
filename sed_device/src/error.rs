//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use super::shared::ata;
use super::shared::nvme;

use super::shared::scsi;
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
    #[error("The drive interface is not supported")]
    InterfaceNotSupported,

    #[error("Security send/receive is not supported by the device")]
    SecurityNotSupported,
    #[error("ATA error: {}", .0)]
    ATAError(ata::ATAError),
    #[error("SCSI error: {}", .0)]
    SCSIError(scsi::SCSIError),
    #[error("NVMe error: {}", .0)]
    NVMeError(nvme::StatusCode),

    #[error("Unspecified error occured (the exact cause could not be determined)")]
    Unspecified,

    #[error("{}", .0)]
    PlatformError(PlatformError),
}
