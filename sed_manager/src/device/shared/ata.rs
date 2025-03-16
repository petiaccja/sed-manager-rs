use sed_manager_macros::Deserialize;

use crate::device::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Command {
    TrustedReceiveDMA = 0x5D,
    TrustedSendDMA = 0x5F,
}

pub struct Input {
    feature: u8,
    count: u8,
    lba: u32,
    command: Command,
}

#[derive(Debug)]
pub struct Output {
    pub interface_crc: bool,
    pub aborted: bool,
    #[allow(unused)]
    pub device_fault: bool,
    #[allow(unused)]
    pub sense_data_available: bool,
    pub error: bool,
}

impl Input {
    pub fn serialize(&self) -> [u8; 8] {
        let lba = self.lba.to_be_bytes();
        [
            self.feature,
            self.count,
            lba[1],
            lba[2],
            lba[3],
            lba[0] & 0x0F,
            self.command as u8,
            0,
        ]
    }

    pub fn trusted_send_dma(
        security_protocol: u8,
        protocol_specific: u16,
        trans_len_bytes: u32,
    ) -> Result<Self, Error> {
        let tf_len_blocks = convert_buffer_len(trans_len_bytes)?;
        let [tf_len_high, tf_len_low] = tf_len_blocks.to_be_bytes();
        let sps = protocol_specific.to_be_bytes();
        let lba = u32::from_be_bytes([0, sps[0], sps[1], tf_len_high]);
        Ok(Self { feature: security_protocol, count: tf_len_low, lba, command: Command::TrustedSendDMA })
    }

    pub fn trusted_receive_dma(
        security_protocol: u8,
        protocol_specific: u16,
        trans_len_bytes: u32,
    ) -> Result<Self, Error> {
        let tf_len_blocks = convert_buffer_len(trans_len_bytes)?;
        let [tf_len_high, tf_len_low] = tf_len_blocks.to_be_bytes();
        let sps = protocol_specific.to_be_bytes();
        let lba = u32::from_be_bytes([0, sps[0], sps[1], tf_len_high]);
        Ok(Self { feature: security_protocol, count: tf_len_low, lba, command: Command::TrustedReceiveDMA })
    }
}

impl Output {
    pub fn parse(registers: [u8; 8]) -> Self {
        let error_reg = registers[0];
        let status_reg = registers[6];
        Self {
            interface_crc: (error_reg & 0b1000_0000) != 0,
            aborted: (error_reg & 0b0000_0100) != 0,
            device_fault: (status_reg & 0b0010_0000) != 0,
            sense_data_available: (status_reg & 0b0000_0010) != 0,
            error: (status_reg & 0b0000_0001) != 0,
        }
    }
}

fn convert_buffer_len(num_bytes: u32) -> Result<u16, Error> {
    if num_bytes % 512 != 0 {
        return Err(Error::InvalidAlignment);
    }
    let num_blocks = num_bytes / 512;
    u16::try_from(num_blocks).map_err(|_| Error::BufferTooLarge)
}

#[derive(Deserialize, Debug, Clone)]
pub struct IdentifyDevice {
    #[layout(offset = 0, bit_field(u16, 15))]
    pub not_ata_device: bool,
    #[layout(offset = 20)]
    pub serial_number: [u8; 20],
    #[layout(offset = 46)]
    pub firmware_revision: [u8; 8],
    #[layout(offset = 54)]
    pub model_number: [u8; 40],
    #[layout(offset = 96, bit_field(u16, 0))]
    pub trusted_computing_supported: bool,
    #[layout(offset = 152)]
    pub serial_ata_capabilities: u16, // This is a bit field, but we only care if it's ATA or SATA.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize() {
        let input = Input { command: Command::TrustedSendDMA, count: 0xAB, feature: 0x89, lba: 0x09AB_CDEF };
        let bytes = input.serialize();
        let expected = [0x89, 0xAB, 0xAB, 0xCD, 0xEF, 0x09, 0x5F, 0];
        assert_eq!(bytes, expected);
    }

    #[test]
    fn trusted_send_dma() {
        let input = Input::trusted_send_dma(0x02, 0x0AFF, 0x3456 * 512).unwrap();
        assert_eq!(input.command, Command::TrustedSendDMA);
        assert_eq!(input.count, 0x56);
        assert_eq!(input.feature, 0x02);
        assert_eq!(input.lba, 0x00_0AFF_34);
    }

    #[test]
    fn trusted_receive_dma() {
        let input = Input::trusted_receive_dma(0x02, 0x0AFF, 0x3456 * 512).unwrap();
        assert_eq!(input.command, Command::TrustedReceiveDMA);
        assert_eq!(input.count, 0x56);
        assert_eq!(input.feature, 0x02);
        assert_eq!(input.lba, 0x00_0AFF_34);
    }
}
