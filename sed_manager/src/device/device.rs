//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use super::error::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interface {
    ATA,
    SATA,
    SCSI,
    NVMe,
    SD,
    MMC,
    Other,
}

pub trait Device: Send + Sync {
    fn path(&self) -> Option<String>;
    fn interface(&self) -> Interface;
    fn model_number(&self) -> String;
    fn serial_number(&self) -> String;
    fn firmware_revision(&self) -> String;

    fn is_security_supported(&self) -> bool;
    fn security_send(&self, security_protocol: u8, protocol_specific: [u8; 2], data: &[u8]) -> Result<(), Error>;
    fn security_recv(&self, security_protocol: u8, protocol_specific: [u8; 2], len: usize) -> Result<Vec<u8>, Error>;
}

impl core::fmt::Display for Interface {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Interface::ATA => write!(f, "ATA"),
            Interface::SATA => write!(f, "SATA"),
            Interface::SCSI => write!(f, "SCSI"),
            Interface::NVMe => write!(f, "NVMe"),
            Interface::SD => write!(f, "SD"),
            Interface::MMC => write!(f, "MMC"),
            Interface::Other => write!(f, "Other"),
        }
    }
}
