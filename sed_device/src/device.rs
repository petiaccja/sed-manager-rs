//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::path::Path;

use crate::error::Error;

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

#[async_trait::async_trait]
pub trait Device: Send + Sync {
    fn path(&self) -> Option<&Path>;
    fn interface(&self) -> Interface;
    fn model_number(&self) -> String;
    fn serial_number(&self) -> String;
    fn firmware_revision(&self) -> String;
    fn is_security_supported(&self) -> bool;

    async fn security_send(&self, security_protocol: u8, protocol_specific: [u8; 2], data: &[u8]) -> Result<(), Error>;
    async fn security_recv(
        &self,
        security_protocol: u8,
        protocol_specific: [u8; 2],
        len: usize,
    ) -> Result<Vec<u8>, Error>;
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

impl<'a> core::fmt::Debug for dyn Device + 'a {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("dyn Device")
            .field("interface", &self.interface())
            .field("model_number", &self.model_number())
            .field("serial_number", &self.serial_number())
            .field("firmware_revision", &self.firmware_revision())
            .field("is_security_supported", &self.is_security_supported())
            .finish()
    }
}
