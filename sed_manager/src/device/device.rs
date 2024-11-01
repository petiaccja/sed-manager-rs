use super::error::Error;

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

pub trait Device {
    fn interface(&self) -> Interface;
    fn model_number(&self) -> Result<String, Error>;
    fn serial_number(&self) -> Result<String, Error>;
    fn firmware_revision(&self) -> Result<String, Error>;

    fn security_send(&self, security_protocol: u8, protocol_specific: [u8; 2], data: &[u8]) -> Result<(), Error>;
    fn security_recv(&self, security_protocol: u8, protocol_specific: [u8; 2], len: usize) -> Result<Vec<u8>, Error>;
}
