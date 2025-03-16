use sed_manager_macros::Deserialize;

use crate::{
    device::{Error as DeviceError, Interface},
    serialization::DeserializeBinary,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Command {
    IdentifyDevice = 0xEC,
    TrustedReceiveDMA = 0x5D,
    TrustedSendDMA = 0x5F,
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

pub struct Input {
    feature: u8,
    count: u8,
    lba: u32,
    command: Command,
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
pub struct StatusField {
    #[layout(offset = 0, bit_field(u8, 7))]
    busy: bool,
    #[layout(offset = 0, bit_field(u8, 6))]
    device_ready: bool,
    #[layout(offset = 0, bit_field(u8, 5))]
    stream_or_device_fault: bool,
    #[layout(offset = 0, bit_field(u8, 4))]
    deferred_write_error: bool,
    #[layout(offset = 0, bit_field(u8, 3))]
    data_request: bool,
    #[layout(offset = 0, bit_field(u8, 2))]
    alignment_error: bool,
    #[layout(offset = 0, bit_field(u8, 1))]
    sense_data_available: bool,
    #[layout(offset = 0, bit_field(u8, 0))]
    error_or_check: bool,
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
pub struct ErrorField {
    #[layout(offset = 0, bit_field(u8, 7))]
    interface_crc: bool,
    #[layout(offset = 0, bit_field(u8, 6))]
    uncorrectable_error: bool,
    #[layout(offset = 0, bit_field(u8, 4))]
    id_not_found: bool,
    #[layout(offset = 0, bit_field(u8, 2))]
    abort: bool,
    #[layout(offset = 0, bit_field(u8, 1))]
    end_of_media: bool,
    #[layout(offset = 0, bit_field(u8, 0))]
    length_or_timeout_or_cfa: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ATAError {
    status: StatusField,
    error: ErrorField,
}

impl IdentifyDevice {
    pub fn interface(&self) -> Interface {
        if self.serial_ata_capabilities != 0 {
            Interface::SATA
        } else {
            Interface::ATA
        }
    }

    pub fn model_number(&self) -> String {
        parse_ata_string(&self.model_number)
    }

    pub fn serial_number(&self) -> String {
        parse_ata_string(&self.serial_number)
    }

    pub fn firmware_revision(&self) -> String {
        parse_ata_string(&self.firmware_revision)
    }
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

    pub fn identify_device() -> Self {
        Self { feature: 0, count: 0, lba: 0, command: Command::IdentifyDevice }
    }

    pub fn trusted_send_dma(
        security_protocol: u8,
        protocol_specific: u16,
        trans_len_bytes: u32,
    ) -> Result<Self, DeviceError> {
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
    ) -> Result<Self, DeviceError> {
        let tf_len_blocks = convert_buffer_len(trans_len_bytes)?;
        let [tf_len_high, tf_len_low] = tf_len_blocks.to_be_bytes();
        let sps = protocol_specific.to_be_bytes();
        let lba = u32::from_be_bytes([0, sps[0], sps[1], tf_len_high]);
        Ok(Self { feature: security_protocol, count: tf_len_low, lba, command: Command::TrustedReceiveDMA })
    }
}

impl ATAError {
    pub fn from_task_file(task_file: [u8; 8]) -> Self {
        let error_reg = task_file[0];
        let status_reg = task_file[6];
        let error = ErrorField::from_bytes([error_reg].into()).unwrap_or(ErrorField::default());
        let status = StatusField::from_bytes([status_reg].into())
            .unwrap_or(StatusField { error_or_check: true, ..Default::default() });
        Self { status, error }
    }

    pub fn with_error_bit() -> Self {
        Self { error: ErrorField::default(), status: StatusField { error_or_check: true, ..Default::default() } }
    }

    pub fn success(&self) -> bool {
        !self.status.error_or_check
    }
}

impl core::error::Error for ATAError {}

impl core::fmt::Display for ATAError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut status = vec![];
        let mut error = vec![];

        if self.status.busy {
            status.push("busy");
        }
        if self.status.device_ready {
            status.push("ready");
        }
        if self.status.stream_or_device_fault {
            status.push("device fault");
        }
        if self.status.deferred_write_error {
            status.push("deferred write error");
        }
        if self.status.data_request {
            status.push("data request");
        }
        if self.status.alignment_error {
            status.push("alignment error");
        }
        if self.status.sense_data_available {
            status.push("sense data available");
        }
        if self.status.error_or_check {
            status.push("error");
        }

        if self.error.interface_crc {
            error.push("interface CRC");
        }
        if self.error.uncorrectable_error {
            error.push("uncorrectable error");
        }
        if self.error.id_not_found {
            error.push("ID not found");
        }
        if self.error.abort {
            error.push("abort");
        }
        if self.error.end_of_media {
            error.push("end of media");
        }
        if self.error.length_or_timeout_or_cfa {
            error.push("length/timeout/CFA");
        }

        let status = status.join(", ");
        let error = error.join(", ");

        write!(f, "Status bits: [{status}], error bits: [{error}]")
    }
}

fn convert_buffer_len(num_bytes: u32) -> Result<u16, DeviceError> {
    if num_bytes % 512 != 0 {
        return Err(DeviceError::InvalidAlignment);
    }
    let num_blocks = num_bytes / 512;
    u16::try_from(num_blocks).map_err(|_| DeviceError::BufferTooLarge)
}

fn parse_ata_string(ata_string: &[u8]) -> String {
    let mut swapped = Vec::from(ata_string);
    for i in (0..swapped.len()).step_by(2) {
        if i + 1 < swapped.len() {
            swapped.swap(i, i + 1);
        }
    }
    String::from_utf8_lossy(&swapped).trim().to_string()
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
