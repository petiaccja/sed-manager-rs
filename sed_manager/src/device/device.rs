use super::{error::Error, get_drive_interface, NVMeDevice};

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

pub trait Device: Send + Sync {
    fn interface(&self) -> Interface;
    fn model_number(&self) -> Result<String, Error>;
    fn serial_number(&self) -> Result<String, Error>;
    fn firmware_revision(&self) -> Result<String, Error>;

    fn security_send(&self, security_protocol: u8, protocol_specific: [u8; 2], data: &[u8]) -> Result<(), Error>;
    fn security_recv(&self, security_protocol: u8, protocol_specific: [u8; 2], len: usize) -> Result<Vec<u8>, Error>;
}

pub fn open_device(drive_path: &str) -> Result<Box<dyn Device>, Error> {
    let interface = get_drive_interface(drive_path)?;
    match interface {
        Interface::ATA => Err(Error::NotSupported),
        Interface::SATA => Err(Error::NotSupported),
        Interface::SCSI => Err(Error::NotSupported),
        Interface::NVMe => NVMeDevice::open(drive_path).map(|device| Box::<dyn Device>::from(Box::from(device))),
        Interface::SD => Err(Error::NotSupported),
        Interface::MMC => Err(Error::NotSupported),
        Interface::Other => Err(Error::NotSupported),
    }
}
