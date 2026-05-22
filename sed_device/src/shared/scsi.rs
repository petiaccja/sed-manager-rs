//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use num_enum::FromPrimitive;
use sorbit::{Deserialize, Serialize, UnpackFrom};

/// A SCSI command.
///
/// When serialized, it produced the command descriptor block (CDB) that can be
/// sent to the SCSI device via IOCTLs.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum Command {
    SecurityProtocolIn(SecurityProtocolIn) = 0xA2,
    SecurityProtocolOut(SecurityProtocolOut) = 0xB5,
}

impl From<SecurityProtocolIn> for Command {
    fn from(value: SecurityProtocolIn) -> Self {
        Self::SecurityProtocolIn(value)
    }
}

impl From<SecurityProtocolOut> for Command {
    fn from(value: SecurityProtocolOut) -> Self {
        Self::SecurityProtocolOut(value)
    }
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[sorbit(byte_order=big_endian)]
pub struct SecurityProtocolIn {
    security_protocol: u8,
    security_protocol_specific: u16,
    #[sorbit(offset = 3, bit_field=_inc_512, repr=u8, bits=7, bit_numbering=LSB0)]
    inc_512: bool,
    #[sorbit(offset = 5)]
    allocation_length: u32,
    #[sorbit(offset = 10)]
    control: u8,
}

impl SecurityProtocolIn {
    pub fn new(security_protocol: u8, security_protocol_specific: u16, alloc_len_bytes: u32, inc_512: bool) -> Self {
        Self {
            security_protocol,
            security_protocol_specific,
            inc_512,
            allocation_length: convert_buffer_len(alloc_len_bytes, inc_512),
            control: 0,
        }
    }
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[sorbit(byte_order=big_endian)]
pub struct SecurityProtocolOut {
    security_protocol: u8,
    security_protocol_specific: u16,
    #[sorbit(offset = 3, bit_field=_inc_512, repr=u8, bits=7, bit_numbering=LSB0)]
    inc_512: bool,
    #[sorbit(offset = 5)]
    transfer_length: u32,
    #[sorbit(offset = 10)]
    control: u8,
}

impl SecurityProtocolOut {
    pub fn new(security_protocol: u8, security_protocol_specific: u16, trans_len_bytes: u32, inc_512: bool) -> Self {
        Self {
            security_protocol,
            security_protocol_specific,
            inc_512,
            transfer_length: convert_buffer_len(trans_len_bytes, inc_512),
            control: 0,
        }
    }
}

/// THe sense data returned by the SCSI device.
///
/// There are two sense data formats:
/// - Fixed format
/// - Descriptor format
///
/// Both formats start with a single byte: `| 7..8: valid | 0..7: response code | ...`.
/// For the fixed format, the valid bit provides additional information about
/// the data structure, for the descriptor format, the valid bit is reserved and
/// has to be zero.
///
/// The lowest bit of the response code encodes whether the sense data is for the
/// current command or a previous one.
///
/// Thus, the valid bit and the current/previous bit create six variants total.
///
/// The sense data is only partially parsed, as most of it is not needed for
/// this application's purposes.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum SenseData {
    CurrentFixed(FixedSenseData) = 0x70,
    DeferredFixed(FixedSenseData) = 0x71,
    CurrentFixedInfo(FixedSenseData) = 0x70 + 0x80,
    DeferredFixedInfo(FixedSenseData) = 0x71 + 0x80,
    CurrentDescriptor(DescriptorSenseData) = 0x72,
    DeferredDescriptor(DescriptorSenseData) = 0x73,
    VendorSpecific = 0x7F,
    #[sorbit(catch_all)]
    Unrecognized(u8),
}

impl SenseData {
    /// The maximum length of the sense data in bytes.
    pub const MAX_LEN: usize = 252;
}

/// Descriptor format sense data.
///
/// The sense response code (first byte) is excluded.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DescriptorSenseData {
    #[sorbit(bit_field=_byte_1, repr=u8, bits=0..=3)]
    pub sense_key: SenseKey,
    pub additional_sense_code: u8,
    pub additional_sense_code_qualifier: u8,
}

/// Fixed format sense data.
///
/// The sense response code (first byte) is excluded.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FixedSenseData {
    #[sorbit(offset = 1, bit_field=_byte_2, repr=u8, bits=0..=3) ]
    pub sense_key: SenseKey,
    #[sorbit(offset = 11)]
    pub additional_sense_code: u8,
    pub additional_sense_code_qualifier: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScsiError {
    Sense { sense_key: SenseKey, additional_sense_code: u8, additional_sense_code_qualifier: u8 },
    Parse,
    VendorSpecific,
    Unknown,
}

impl ScsiError {
    pub fn ok() -> Self {
        Self::Sense { sense_key: SenseKey::NoSense, additional_sense_code: 0, additional_sense_code_qualifier: 0 }
    }
}

impl core::error::Error for ScsiError {}

impl core::fmt::Display for ScsiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScsiError::Sense { sense_key, additional_sense_code, additional_sense_code_qualifier } => {
                write!(f, "{} [ASC={}h ASCQ={}h]", sense_key, additional_sense_code, additional_sense_code_qualifier)
            }
            ScsiError::Parse => write!(f, "failed to parse SCSI sense info"),
            ScsiError::VendorSpecific => write!(f, "a vendor-specific SCSI error occured"),
            ScsiError::Unknown => write!(f, "an unknown SCSI error occured"),
        }
    }
}

impl Default for ScsiError {
    fn default() -> Self {
        Self::ok()
    }
}

impl From<SenseData> for ScsiError {
    fn from(value: SenseData) -> Self {
        match value {
            SenseData::CurrentFixed(FixedSenseData {
                sense_key,
                additional_sense_code,
                additional_sense_code_qualifier,
            })
            | SenseData::CurrentFixedInfo(FixedSenseData {
                sense_key,
                additional_sense_code,
                additional_sense_code_qualifier,
            })
            | SenseData::CurrentDescriptor(DescriptorSenseData {
                sense_key,
                additional_sense_code,
                additional_sense_code_qualifier,
            }) => Self::Sense { sense_key, additional_sense_code, additional_sense_code_qualifier },
            SenseData::DeferredFixed(_) => Self::ok(),
            SenseData::DeferredFixedInfo(_) => Self::ok(),
            SenseData::DeferredDescriptor(_) => Self::ok(),
            SenseData::VendorSpecific => Self::VendorSpecific,
            SenseData::Unrecognized(_) => Self::Unknown,
        }
    }
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, thiserror::Error, UnpackFrom)]
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
    #[error(
        "Blank check: a write-once device or a sequential-access device encountered blank medium or format-defined end-of-data"
    )]
    BlankCheck = 0x8,
    #[error("Vendor specific: the sense data is vendor specific")]
    VendorSpecific = 0x9,
    #[error("Copy aborted: an EXTENDED COPY command was aborted")]
    AbortedCopy = 0xA,
    #[error("Aborted command: the device server aborted the command")]
    AbortedCommand = 0xB,
    #[error(
        "Volume overflow: a buffered SCSI device has reached the end-of-partition and data may remain in the buffer that has not been written to the medium"
    )]
    VolumeOverflow = 0xD,
    #[error("Miscompare: the source data did not match the data read from the medium")]
    Miscompare = 0xE,
    #[error("Reserved sense key")]
    Reserved = 0xF,
}

fn convert_buffer_len(num_bytes: u32, inc_512: bool) -> u32 {
    if inc_512 {
        assert_eq!(num_bytes % 512, 0);
        num_bytes / 512
    } else {
        num_bytes
    }
}

#[cfg(test)]
mod tests {
    use sorbit::ser_de::{FromBytes as _, ToBytes as _};

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

    #[test]
    fn serialize_security_protocol_in() {
        let bytes = [
            0xA2, 0x12, 0x34, 0x56, 0x80, 0x00, 0x12, 0x34, 0xAB, 0xCD, 0x00, 0x56,
        ];
        let value = Command::SecurityProtocolIn(SecurityProtocolIn {
            security_protocol: 0x12,
            security_protocol_specific: 0x3456,
            inc_512: true,
            allocation_length: 0x1234ABCD,
            control: 0x56,
        });
        assert_eq!(&value.to_bytes().unwrap(), &bytes);
    }

    #[test]
    fn serialize_security_protocol_out() {
        let bytes = [
            0xB5, 0x12, 0x34, 0x56, 0x80, 0x00, 0x12, 0x34, 0xAB, 0xCD, 0x00, 0x56,
        ];
        let value = Command::SecurityProtocolOut(SecurityProtocolOut {
            security_protocol: 0x12,
            security_protocol_specific: 0x3456,
            inc_512: true,
            transfer_length: 0x1234ABCD,
            control: 0x56,
        });
        assert_eq!(&value.to_bytes().unwrap(), &bytes);
    }

    #[test]
    fn deserialize_descriptor_sense_data() {
        let bytes = [0x72, 0x02, 0x12, 0x34, 0x00, 0x00, 0x56];
        let value = SenseData::CurrentDescriptor(DescriptorSenseData {
            sense_key: SenseKey::NotReady,
            additional_sense_code: 0x12,
            additional_sense_code_qualifier: 0x34,
        });
        assert_eq!(value, SenseData::from_bytes(&bytes).unwrap());
    }

    #[test]
    fn deserialize_fixed_sense_data() {
        let mut bytes = [0x00_u8; 18];
        bytes[0] = 0x70;
        bytes[2] = 0x02;
        bytes[12] = 0x12;
        bytes[13] = 0x34;

        let value = SenseData::CurrentFixed(FixedSenseData {
            sense_key: SenseKey::NotReady,
            additional_sense_code: 0x12,
            additional_sense_code_qualifier: 0x34,
        });
        assert_eq!(value, SenseData::from_bytes(&bytes).unwrap());
    }
}
