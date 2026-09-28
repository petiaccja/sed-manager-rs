//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

//! Implements parts of the NVMe specification that is relevant for drive encryption.
//! The official specification is accessible on [NVMe's website](https://nvmexpress.org/specifications/).

use crate::Error as DeviceError;
use num_enum::{FromPrimitive, IntoPrimitive};
use sorbit::{Deserialize, PackInto, UnpackFrom, ser_de::FromBytes as _, ser_de::ToBytes as _};

/// NVMe opcodes. These are combined opcodes, containing both the function and the data transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    Identify = 0x06,
    SecuritySend = 0x81,
    SecurityReceive = 0x82,
    /// Send an invalid command to the NVMe controller to test error handling.
    Invalid = 0b101111_00,
}

/// The data structure returned by the Identify Admin command called on the controller.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[sorbit(byte_order=little_endian)]
pub struct IdentifyController {
    pub vendor_id: u16,
    pub subsystem_vendor_id: u16,
    pub serial_number: [u8; 20],
    pub model_number: [u8; 40],
    pub firmware_revision: [u8; 8],
    pub recommended_arbitration_burst: u8,
    pub ieee_oui_identifier: [u8; 3],
    #[sorbit(bit_field=_oacs, repr=u16, offset=256, bit_numbering=LSB0)]
    #[sorbit(bits = 0)]
    pub security_send_receive_supported: bool,
}

/// The data structure returned by the Identify Admin command called on a namespace.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[sorbit(byte_order=little_endian)]
pub struct IdentifyNamespace {
    pub namespace_size: u64,
    pub namespace_capacity: u64,
    pub namespace_utiliziation: u64,
    pub namespace_features: u8,
    pub num_common_lba_formats: u8,

    #[sorbit(bit_field=formatted_lba_size, repr=u8, bits = 5..=6)]
    pub lba_format_index_upper: u8,
    #[sorbit(bit_field=formatted_lba_size, bits = 4)]
    pub lba_metadata_tx_as_ext_lba: bool,
    #[sorbit(bit_field=formatted_lba_size, bits = 0..=3)]
    pub lba_format_index_lower: u8,

    #[sorbit(offset = 82)]
    pub num_uncommon_lba_formats: u8,

    #[sorbit(offset = 128)]
    pub lba_formats: [LbaFormat; 64],
}

impl IdentifyNamespace {
    /// Return the total number of LBA formats the namespace supports. This is
    /// the sum of the common and unique attribute format counts.
    ///
    /// The number of common formats is a 0's based number, so a value of 0 means
    /// 1. One is therefore added to the sum.
    pub fn num_lba_formats(&self) -> u8 {
        self.num_common_lba_formats + 1 + self.num_uncommon_lba_formats
    }

    /// Returns the namespace's current LBA format.
    ///
    /// # Errors
    ///
    /// This function can technically fail if the namespace data returned by the
    /// drive is not correct. This is very unlikely.
    pub fn lba_format(&self) -> Option<&LbaFormat> {
        let num_lba_formats = self.num_lba_formats();
        let lba_format_index = match num_lba_formats {
            0..=16 => self.lba_format_index_lower,
            17.. => (self.lba_format_index_upper << 4) + self.lba_format_index_lower,
        };
        (lba_format_index < num_lba_formats).then(|| &self.lba_formats[usize::from(lba_format_index)])
    }
}

/// Describe an LBA formatting scheme supported by the NVMe device.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[sorbit(byte_order=little_endian)]
pub struct LbaFormat {
    #[sorbit(bit_field = _0, bit_numbering = lsb0, repr = u32, bits = 24..=25)]
    relative_performance: RelativePerformance,
    #[sorbit(bit_field = _0, bits = 16..=23)]
    log_logical_sector_size: u8,
    #[sorbit(bit_field = _0, bits = 0..=15)]
    metadata_size: u16,
}

impl LbaFormat {
    /// If the log(logical sector size) is zero, the format is not available even
    /// though it's otherwise supported by the drive.
    pub fn is_available(&self) -> bool {
        self.log_logical_sector_size != 0
    }

    /// Returns the logical sector size (not it's logarithm) if the format is available.
    pub fn logical_sector_size(&self) -> Option<u32> {
        match self.log_logical_sector_size {
            0 => None, // The format is not currently available.
            log_logical_sector_size @ 1..=32 => Some(1 << log_logical_sector_size),
            33.. => None, // Sectors over 4 GiB are probably not supported.
        }
    }
}

#[derive(Deserialize, UnpackFrom, Clone, Debug, PartialEq, Eq)]
#[sorbit(byte_order=little_endian)]
#[repr(u8)]
pub enum RelativePerformance {
    Best = 0b00,
    Better = 0b01,
    Good = 0b10,
    Degraded = 0b11,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[sorbit(byte_order=little_endian)]
pub struct StatusField {
    #[sorbit(bit_field=_all, repr=u32, bit_numbering=LSB0)]
    #[sorbit(bits = 31)]
    do_not_retry: bool,
    #[sorbit(bit_field=_all, bits=30)]
    more: bool,
    #[sorbit(bit_field=_all, bits=28..=29)]
    retry_delay: u8,
    #[sorbit(bit_field=_all, bits=25..=27)]
    status_code_type: StatusCodeType,
    #[sorbit(bit_field=_all, bits=17..=24)]
    status_code: u8,
}

/// NVMe status codes. These indicate the success/failure of an NVMe command.
/// [`StatusCode`] contains both the status code type and the status code value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusCode {
    Generic(GenericStatusCode),
    CommandSpecific(u8),
    MediaIntegrity(u8),
    PathRelated(u8),
    Unknown(u8),
    InvalidStatusField,
}

impl core::fmt::Display for StatusCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StatusCode::Generic(code) => write!(f, "{code} (type=0h, code={:02x}h)", u8::from(*code)),
            StatusCode::CommandSpecific(code) => write!(f, "Command specific error (type=1h, code={:02x}h)", code),
            StatusCode::MediaIntegrity(code) => write!(f, "Media integrity error (type=2h, code={:02x}h)", code),
            StatusCode::PathRelated(code) => write!(f, "Path related error (type=3h, code={:02x}h)", code),
            StatusCode::Unknown(code) => write!(f, "Unknown error (type=4h-7h, code={:02x}h)", code),
            StatusCode::InvalidStatusField => write!(f, "Invalid status field"),
        }
    }
}

impl core::error::Error for StatusCode {}

/// NVMe status code types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PackInto, UnpackFrom)]
#[repr(u8)]
pub enum StatusCodeType {
    Generic = 0x0,
    CommandSpecific = 0x1,
    MediaIntegrity = 0x2,
    PathRelated = 0x3,
    Unknown = 0xFF,
}

/// Exhaustive list of generic status code values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error, IntoPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum GenericStatusCode {
    #[error("The command completed successfully")]
    Success = 0x00,
    #[error("Invalid (reserved or unsupported) command opcode")]
    InvalidCommandOpcode = 0x01,
    #[error("Invalid command parameter or invalid parameter in structures pointer to by command parameters")]
    InvalidCommandParameter = 0x02,
    #[error("Command ID conflict: the command identifier is already in use")]
    CommandIDConflict = 0x03,
    #[error("Data transfer error: transferring the data or metadata associated with a command had an error")]
    DataTransferError = 0x04,
    #[error("Commands aborted due to power loss notification")]
    AbortPowerLoss = 0x05,
    #[error("Internal error: the command failed due to an internal error")]
    InternalError = 0x06,
    #[error("Command aborted due to SQ deletion")]
    AbortSQDeletion = 0x08,
    #[error("Command aborted due to failed fused fommand")]
    AbortFailedFusedCommand = 0x09,
    #[error("Command aborted due to missing fused command")]
    AbortMissingFusedCommand = 0x0A,
    #[error("Invalid namespace or namespace format")]
    InvalidNamespace = 0x0B,
    #[error("Command sequence error: e.g., a violation of the Security Send and Security Receive sequencing rules")]
    CommandSequenceError = 0x0C,
    #[error("Invalid SGL segment descriptor")]
    InvalidSGLSegmentDesc = 0x0D,
    #[error("Invalid number of SGL descriptors")]
    InvalidNumSGLDescs = 0x0E,
    #[error("Data SGL length invalid")]
    InvalidDataSGLLen = 0x0F,
    #[error("Metadata SGL length invalid")]
    InvalidMetadataSGLLen = 0x10,
    #[error("SGL descriptor type invalid")]
    InvalidSGLDescType = 0x11,
    #[error("Invalid use of controller memory buffer")]
    InvalidBufferUse = 0x12,
    #[error("PRP offset invalid")]
    InvalidPRPOffset = 0x13,
    #[error("Atomic write unit exceeded")]
    AtomicWriteUnitExceeded = 0x14,
    #[error("Operation denied: the command was denied due to lack of access rights")]
    AccessDenied = 0x15,
    #[error("SGL offset invalid")]
    InvalidSGLOffset = 0x16,
    #[error(
        "Host identifier inconsistent format: the NVM subsystem detected the simultaneous use of 64-bit and 128-bit Host Identifier values on different controllers"
    )]
    InconsistentHostIdentifier = 0x18,
    #[error("Keep alive timer expired")]
    KeepAliveExpored = 0x19,
    #[error("Keep alive timeout invalid")]
    KeepAliveInvalid = 0x1A,
    #[error("Command aborted due to preempt and abort")]
    AbortPreempt = 0x1B,
    #[error("Sanitize failed and no recovery action has been successfully completed")]
    SanitizeFailed = 0x1C,
    #[error("Sanitize in progress: the requested function is prohibited while a sanitize operation is in progress")]
    SanitizeInProgress = 0x1D,
    #[error("SGL data block granularity invalid")]
    InvalidSGLGranularity = 0x1E,
    #[error("Command not supported for queue in CMB")]
    CommandNotSupportedForCMB = 0x1F,
    #[error("Namespace is write protected: the command is prohibited while the namespace is write protected")]
    NamespaceWriteProtected = 0x20,
    #[error(
        "Command interrupted: command processing was interrupted and the controller is unable to successfully complete the command"
    )]
    Interrupted = 0x21,
    #[error("Transient ASDacement Handle List")]
    InvalidPlacementHandleList = 0x2A,
    #[error("LBA out of range")]
    LBAOutOfRange = 0x80,
    #[error("Capacity exceeded: the command attempted an operation that exceeds the capacity of the namespace")]
    CapacityExceeded = 0x81,
    #[error("Namespace not ready: the namespace is not ready to be accessed")]
    NamespaceNotReady = 0x82,
    #[error(
        "Reservation conflict: the command was aborted due to a conflict with a reservation held on the accessed namespace"
    )]
    ReservationConflict = 0x83,
    #[error("Format in progress: a Format NVM command is in progress on the namespace")]
    FormatInProgress = 0x84,
    #[error("Invalid value size")]
    InvalidValueSize = 0x85,
    #[error("Invalid key size")]
    InvalidKeySize = 0x86,
    #[error("KV key does not exist")]
    KVDoesNotExist = 0x87,
    #[error("Unrecovered error")]
    UnrecoveredError = 0x88,
    #[error("Key exists")]
    KeyExists = 0x89,
    #[error("Unrecognized error: {0:02x}h")]
    #[num_enum(catch_all)]
    Unrecognized(u8),
}

impl IdentifyController {
    pub fn serial_number_as_str(&self) -> String {
        String::from_utf8_lossy(&self.serial_number).trim().to_string()
    }
    pub fn model_number_as_str(&self) -> String {
        String::from_utf8_lossy(&self.model_number).trim().to_string()
    }
    pub fn firmware_revision_as_str(&self) -> String {
        String::from_utf8_lossy(&self.firmware_revision).trim().to_string()
    }
}

impl StatusField {
    pub fn status_code(&self) -> StatusCode {
        let status_code = self.status_code;
        match self.status_code_type {
            StatusCodeType::Generic => StatusCode::Generic(GenericStatusCode::from(status_code)),
            StatusCodeType::CommandSpecific => StatusCode::CommandSpecific(status_code),
            StatusCodeType::MediaIntegrity => StatusCode::MediaIntegrity(status_code),
            StatusCodeType::PathRelated => StatusCode::PathRelated(status_code),
            _ => StatusCode::Unknown(status_code),
        }
    }
}

#[cfg(test)]
mod tests {
    use googletest::{assert_that, matchers::*};

    use super::*;

    #[test]
    fn status_code_from_integer_generic() {
        let encoded = 0b0_0_00_000_00000001_0_0000_0000_0000_0000_u32; // First bit should be ignored.
        let status = StatusField::from_bytes(&encoded.to_le_bytes()).unwrap().status_code();
        assert_eq!(status, StatusCode::Generic(GenericStatusCode::InvalidCommandOpcode));
    }

    #[test]
    fn status_code_from_integer_cmd_specific() {
        let encoded = 0b0_0_00_001_00000001_0_0000_0000_0000_0000_u32;
        let status = StatusField::from_bytes(&encoded.to_le_bytes()).unwrap().status_code();
        assert_eq!(status, StatusCode::CommandSpecific(1));
    }

    #[test]
    fn serialization_identify_controller() -> Result<(), Box<dyn std::error::Error>> {
        let content = IdentifyController {
            vendor_id: 0x1234,
            subsystem_vendor_id: 0x5678,
            serial_number: *b"123                 ",
            model_number: *b"456                                     ",
            firmware_revision: *b"789     ",
            recommended_arbitration_burst: 0x12,
            ieee_oui_identifier: [0x34, 0x56, 0x67],
            security_send_receive_supported: true,
        };
        let bytes: Vec<_> = [
            &[0x34, 0x12],
            &[0x78, 0x56],
            b"123                 ".as_slice(),
            b"456                                     ".as_slice(),
            b"789     ".as_slice(),
            &[0x12],
            &[0x34, 0x56, 0x67],
            &[0x00; 180],
            &[0x01, 0x00],
        ]
        .iter()
        .flat_map(|x| x.iter())
        .cloned()
        .collect();
        assert_eq!(IdentifyController::from_bytes(bytes.as_ref())?, content);
        Ok(())
    }

    #[test]
    fn serialization_identify_namespace() -> Result<(), Box<dyn std::error::Error>> {
        let content = IdentifyNamespace {
            namespace_size: 0x1234,
            namespace_capacity: 0x1235,
            namespace_utiliziation: 0x1236,
            namespace_features: 0xE5,
            num_common_lba_formats: 0x08,
            lba_format_index_upper: 0x00,
            lba_metadata_tx_as_ext_lba: false,
            lba_format_index_lower: 0x06,
            num_uncommon_lba_formats: 0x03,
            lba_formats: std::array::from_fn(|i| LbaFormat {
                relative_performance: RelativePerformance::Good,
                log_logical_sector_size: 9 + i as u8 % 4,
                metadata_size: i as u16,
            }),
        };

        fn lba_format_array_byte(byte_idx: usize) -> u8 {
            let idx = byte_idx / 4;
            match byte_idx % 4 {
                0 => idx as u8,
                1 => (idx as u16 >> 8) as u8,
                2 => 9 + idx as u8 % 4,
                _ => 0b0000_0010, // Performance: Good
            }
        }

        let bytes: Vec<_> = [
            [0x34, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00].as_slice(), // NSZE
            [0x35, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00].as_slice(), // NCAP
            [0x36, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00].as_slice(), // NUSE
            [0xE5].as_slice(),                                           // NSFEAT
            [0x08].as_slice(),                                           // NLBAF
            [0b0_00_0_0110].as_slice(),                                  // FLBAS
            [0; 55].as_slice(),                                          // Padding for 27..82
            [0x03].as_slice(),                                           // NULBAF
            [0; 45].as_slice(),                                          // Padding for 83..128
            std::array::from_fn::<u8, 256, _>(|i| lba_format_array_byte(i)).as_slice(), // LBAF0..=LBAF63
        ]
        .iter()
        .flat_map(|x| x.iter())
        .cloned()
        .collect();

        assert_that!(IdentifyNamespace::from_bytes(bytes.as_ref())?, eq(&content));
        Ok(())
    }

    #[test]
    fn namespace_identity_properties_regular() {
        let identity = IdentifyNamespace {
            namespace_size: 123,
            namespace_capacity: 123,
            namespace_utiliziation: 105,
            namespace_features: 0xE5,
            num_common_lba_formats: 7,
            lba_format_index_upper: 0,
            lba_metadata_tx_as_ext_lba: false,
            lba_format_index_lower: 6,
            num_uncommon_lba_formats: 3,
            lba_formats: std::array::from_fn(|i| LbaFormat {
                relative_performance: RelativePerformance::Good,
                log_logical_sector_size: 9 + i as u8 % 4,
                metadata_size: i as u16,
            }),
        };

        assert_that!(identity.num_lba_formats(), eq(11));
        assert_that!(
            identity.lba_format(),
            some(eq(&LbaFormat {
                relative_performance: RelativePerformance::Good,
                log_logical_sector_size: 11,
                metadata_size: 6
            }))
        );
    }

    #[test]
    fn namespace_identity_properties_single_format() {
        let identity = IdentifyNamespace {
            namespace_size: 123,
            namespace_capacity: 123,
            namespace_utiliziation: 105,
            namespace_features: 0xE5,
            num_common_lba_formats: 0,
            lba_format_index_upper: 0,
            lba_metadata_tx_as_ext_lba: false,
            lba_format_index_lower: 0,
            num_uncommon_lba_formats: 0,
            lba_formats: std::array::from_fn(|i| LbaFormat {
                relative_performance: RelativePerformance::Good,
                log_logical_sector_size: if i == 0 { 9 } else { 0 },
                metadata_size: 0,
            }),
        };

        assert_that!(identity.num_lba_formats(), eq(1));
        assert_that!(
            identity.lba_format(),
            some(eq(&LbaFormat {
                relative_performance: RelativePerformance::Good,
                log_logical_sector_size: 9,
                metadata_size: 0
            }))
        );
    }

    #[test]
    fn lba_format_properties_available() {
        let format = LbaFormat {
            relative_performance: RelativePerformance::Good,
            log_logical_sector_size: 11,
            metadata_size: 6,
        };
        assert_that!(format.is_available(), eq(true));
        assert_that!(format.logical_sector_size(), some(eq(2048)));
    }

    #[test]
    fn lba_format_properties_unavailable() {
        let format =
            LbaFormat { relative_performance: RelativePerformance::Good, log_logical_sector_size: 0, metadata_size: 6 };
        assert_that!(format.is_available(), eq(false));
        assert_that!(format.logical_sector_size(), none());
    }
}
