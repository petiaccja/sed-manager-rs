use sed_manager_macros::Deserialize;

use crate::serialization::Serialize;

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Opcode {
    SecurityProtocolOut = 0xB5,
    SecurityProtocolIn = 0xA2,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct SecurityProtocolIn {
    opcode: Opcode,
    security_protocol: u8,
    security_protocol_specific: u16,
    #[layout(offset = 4, bit_field(u8, 7))]
    inc_512: bool,
    #[layout(offset = 6)]
    allocation_length: u32,
    #[layout(offset = 11)]
    control: u8,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct SecurityProtocolOut {
    opcode: Opcode,
    security_protocol: u8,
    security_protocol_specific: u16,
    #[layout(offset = 4, bit_field(u8, 7))]
    inc_512: bool,
    #[layout(offset = 6)]
    transfer_length: u32,
    #[layout(offset = 11)]
    control: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SCSIError {
    pub sense_key: SenseKey,
    pub additional_sense_code: u8,
    pub additional_sense_code_qualifier: u8,
    pub parse_failed: bool,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SenseResponseCode {
    CurrentFixed = 0x70,
    DeferredFixed = 0x71,
    CurrentDescriptor = 0x72,
    DeferredDescriptor = 0x73,
    VendorSpecific = 0x7F,
    #[fallback]
    Unrecognized,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[repr(u8)]
pub enum SenseKey {
    #[error("No sense: no specific sense key information to be reported")]
    NoSense = 0x0,
    #[error("Recovered error: command was successful, but error recovery was performed")]
    RecoveredError = 0x1,
    #[error("Not ready: the logical unit is not accessible")]
    NotReady = 0x2,
    #[error("Medium error: flaw in the medium or the recorded data")]
    MediumError = 0x3,
    #[error("Hardware error: e.g. controller failure, parity error")]
    HardwareError = 0x4,
    #[error("Illegal request: incorrect parameters in the Command Descriptor Block")]
    IllegalRequest = 0x5,
    #[error("Unit attention: a unit attention condition has been established (e.g. removed medium)")]
    UnitAttention = 0x6,
    #[error("Data protect: the read/written block is protected")]
    DataProtect = 0x7,
    #[error("Blank check: a write-once device or a sequential-access device encountered blank medium or format-defined end-of-data")]
    BlankCheck = 0x8,
    #[error("Vendor specific: the sense data is vendor specific")]
    VendorSpecific = 0x9,
    #[error("Copy aborted: an EXTENDED COPY command was aborted")]
    AbortedCopy = 0xA,
    #[error("Aborted command: the device server aborted the command")]
    AbortedCommand = 0xB,
    #[error("Volume overflow: a buffered SCSI device has reached the end-of-partition and data may remain in the buffer that has not been written to the medium")]
    VolumeOverflow = 0xD,
    #[error("Miscompare: the source data did not match the data read from the medium")]
    Miscompare = 0xE,
    #[error("Reserved sense key")]
    Reserved = 0xF,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DescriptorSenseData {
    #[layout(offset = 0, bit_field(u8, 0..=6)) ]
    pub response_code: SenseResponseCode,
    #[layout(offset = 1, bit_field(u8, 0..=3)) ]
    pub sense_key: SenseKey,
    pub additional_sense_code: u8,
    pub additional_sense_code_qualifier: u8,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FixedSenseData {
    #[layout(offset = 0, bit_field(u8, 0..=6)) ]
    pub response_code: SenseResponseCode,
    #[layout(offset = 2, bit_field(u8, 0..=3)) ]
    pub sense_key: SenseKey,
    #[layout(offset = 12)]
    pub additional_sense_code: u8,
    pub additional_sense_code_qualifier: u8,
}

fn convert_buffer_len(num_bytes: u32, inc_512: bool) -> u32 {
    if inc_512 {
        assert_eq!(num_bytes % 512, 0);
        num_bytes / 512
    } else {
        num_bytes
    }
}

impl SecurityProtocolIn {
    pub fn new(security_protocol: u8, security_protocol_specific: u16, alloc_len_bytes: u32, inc_512: bool) -> Self {
        Self {
            opcode: Opcode::SecurityProtocolIn,
            security_protocol,
            security_protocol_specific,
            inc_512,
            allocation_length: convert_buffer_len(alloc_len_bytes, inc_512),
            control: 0,
        }
    }
}

impl SecurityProtocolOut {
    pub fn new(security_protocol: u8, security_protocol_specific: u16, trans_len_bytes: u32, inc_512: bool) -> Self {
        Self {
            opcode: Opcode::SecurityProtocolOut,
            security_protocol,
            security_protocol_specific,
            inc_512,
            transfer_length: convert_buffer_len(trans_len_bytes, inc_512),
            control: 0,
        }
    }
}

impl core::error::Error for SCSIError {}

impl core::fmt::Display for SCSIError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.parse_failed {
            write!(
                f,
                "{} [ASC={}h ASCQ={}h]",
                self.sense_key, self.additional_sense_code, self.additional_sense_code_qualifier
            )
        } else {
            write!(f, "Failed to parse Sense Info")
        }
    }
}

impl Default for SCSIError {
    fn default() -> Self {
        Self {
            sense_key: SenseKey::NoSense,
            additional_sense_code: 0,
            additional_sense_code_qualifier: 0,
            parse_failed: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn security_protocol_out_new_bytes() {
        let value = SecurityProtocolOut::new(0, 0, 235, false);
        assert_eq!(value.transfer_length, 235);
        assert_eq!(value.inc_512, false);
    }

    #[test]
    fn security_protocol_out_new_512_ok() {
        let value = SecurityProtocolOut::new(0, 0, 512, true);
        assert_eq!(value.transfer_length, 1);
        assert_eq!(value.inc_512, true);
    }

    #[test]
    #[should_panic]
    fn security_protocol_out_new_512_err() {
        let _ = SecurityProtocolOut::new(0, 0, 235, true);
    }

    #[test]
    fn security_protocol_in_new_bytes() {
        let value = SecurityProtocolIn::new(0, 0, 235, false);
        assert_eq!(value.allocation_length, 235);
        assert_eq!(value.inc_512, false);
    }

    #[test]
    fn security_protocol_in_new_512_ok() {
        let value = SecurityProtocolIn::new(0, 0, 512, true);
        assert_eq!(value.allocation_length, 1);
        assert_eq!(value.inc_512, true);
    }

    #[test]
    #[should_panic]
    fn security_protocol_in_new_512_err() {
        let _ = SecurityProtocolIn::new(0, 0, 235, true);
    }
}
