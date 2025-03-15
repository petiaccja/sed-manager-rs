use crate::serialization::{Deserialize, Serialize};

// NVMe specification:
// https://nvmexpress.org/wp-content/uploads/NVM-Express-Base-Specification-2_0-2021.06.02-Ratified-5.pdf

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    IdentifyController = 0x06, // Not needed for Windows due to specific IOCTL.
    SecuritySend = 0x81,
    SecurityReceive = 0x82,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[layout(little_endian)]
pub struct IdentifyController {
    pub vendor_id: u16,
    pub subsystem_vendor_id: u16,
    pub serial_number: [u8; 20],
    pub model_number: [u8; 40],
    pub firmware_revision: [u8; 8],
    pub recommended_arbitration_burst: u8,
    pub ieee_oui_identifier: [u8; 3],
}

impl IdentifyController {
    pub fn serial_number_as_str(&self) -> String {
        String::from_utf8_lossy(&self.serial_number).to_string()
    }
    pub fn model_number_as_str(&self) -> String {
        String::from_utf8_lossy(&self.model_number).to_string()
    }
    pub fn firmware_revision_as_str(&self) -> String {
        String::from_utf8_lossy(&self.firmware_revision).to_string()
    }
}
