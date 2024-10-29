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
use super::windows::Error as OSError;

#[cfg(target_os = "linux")]
use super::windows::Linux as OSError;

pub enum DeviceError {
    BufferTooLarge,
    NotFound,
    OSError(OSError),
}

pub trait Device {
    fn interface(&self) -> Interface;
    fn model_number(&self) -> Result<String, DeviceError>;
    fn serial_number(&self) -> Result<String, DeviceError>;
    fn firmware_revision(&self) -> Result<String, DeviceError>;

    fn security_send(&self, security_protocol: u8, protocol_specific: [u8; 2], data: &[u8]) -> Result<(), DeviceError>;
    fn security_recv(
        &self,
        security_protocol: u8,
        protocol_specific: [u8; 2],
        len: usize,
    ) -> Result<Vec<u8>, DeviceError>;
}

impl Display for DeviceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceError::BufferTooLarge => f.write_fmt(format_args!("buffer too large")),
            DeviceError::NotFound => f.write_fmt(format_args!("not found")),
            DeviceError::OSError(err) => f.write_fmt(format_args!("{}", err)),
        }
    }
}
