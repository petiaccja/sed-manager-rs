#[derive(Debug, PartialEq, Eq)]
pub enum Interface {
    ATA,
    SATA,
    SCSI,
    NVMe,
    SD,
    MMC,
    Other,
}

use std::fmt::Display;

#[cfg(target_os = "windows")]
use super::windows::Error as PlatformError;

#[cfg(target_os = "linux")]
use super::linux::Error as PlatformError;

#[derive(Debug)]
pub enum Error {
    DataTooLong,
    DeviceNotFound,
    InvalidArgument,
    NotSupported,
    PermissionDenied,
    Source(PlatformError),
}

impl std::error::Error for Error {}

pub trait Device {
    fn interface(&self) -> Interface;
    fn model_number(&self) -> Result<String, Error>;
    fn serial_number(&self) -> Result<String, Error>;
    fn firmware_revision(&self) -> Result<String, Error>;

    fn security_send(&self, security_protocol: u8, protocol_specific: [u8; 2], data: &[u8]) -> Result<(), Error>;
    fn security_recv(&self, security_protocol: u8, protocol_specific: [u8; 2], len: usize) -> Result<Vec<u8>, Error>;
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::DataTooLong => f.write_fmt(format_args!("provided data is too long")),
            Error::DeviceNotFound => f.write_fmt(format_args!("could not find the specified device")),
            Error::InvalidArgument => f.write_fmt(format_args!("invalid argument")),
            Error::NotSupported => f.write_fmt(format_args!("not supported")),
            Error::PermissionDenied => f.write_fmt(format_args!("permission denied")),
            Error::Source(err) => f.write_fmt(format_args!("{}", err)),
        }
    }
}
