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
    fn interface(&self) -> Result<Interface, Error>;
    fn model_number(&self) -> Result<String, Error>;
    fn serial_number(&self) -> Result<String, Error>;
    fn firmware_revision(&self) -> Result<String, Error>;

    fn security_send(&self, security_protocol: u8, protocol_specific: [u8; 2], data: &[u8]) -> Result<(), Error>;
    fn security_recv(&self, security_protocol: u8, protocol_specific: [u8; 2], len: usize) -> Result<Vec<u8>, Error>;
}

impl std::fmt::Display for Interface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
