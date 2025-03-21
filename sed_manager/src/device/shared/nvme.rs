//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

//! Implements parts of the NVMe specification that is relevant for drive encryption.
//! The official specification is accessible on [NVMe's website](https://nvmexpress.org/specifications/).

use crate::device::Error as DeviceError;
use crate::serialization::{Deserialize, DeserializeBinary, Serialize};

/// NVMe opcodes. These are combined opcodes, containing both the function and the data transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    IdentifyController = 0x06,
    SecuritySend = 0x81,
    SecurityReceive = 0x82,
    /// Send an invalid command to the NVMe controller to test error handling.
    Invalid = 0b101111_00,
}

/// The data structure returned by the Identify controller Admin command.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[layout(little_endian)]
pub struct IdentifyController {
    pub vendor_id: u16,
    pub subsystem_vendor_id: u16,
    pub serial_number: [u8; 20],
    pub model_number: [u8; 40],
    pub firmware_revision: [u8; 8],
    pub recommended_arbitration_burst: u8,
    pub ieee_oui_identifier: [u8; 3],
    #[layout(offset = 256, bit_field(u16, 0))]
    pub security_send_receive_supported: bool,
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

/// NVMe status code types.
#[repr(u8)]
pub enum StatusCodeType {
    Generic = 0x0,
    CommandSpecific = 0x1,
    MediaIntegrity = 0x2,
    PathRelated = 0x3,
    Unknown = 0xFF,
}

/// Exhaustive list of generic status code values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, thiserror::Error)]
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
    #[error("Host identifier inconsistent format: the NVM subsystem detected the simultaneous use of 64-bit and 128-bit Host Identifier values on different controllers")]
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
    #[error("Command interrupted: command processing was interrupted and the controller is unable to successfully complete the command")]
    Interrupted = 0x21,
    #[error("Transient ASDacement Handle List")]
    InvalidPlacementHandleList = 0x2A,
    #[error("LBA out of range")]
    LBAOutOfRange = 0x80,
    #[error("Capacity exceeded: the command attempted an operation that exceeds the capacity of the namespace")]
    CapacityExceeded = 0x81,
    #[error("Namespace not ready: the namespace is not ready to be accessed")]
    NamespaceNotReady = 0x82,
    #[error("Reservation conflict: the command was aborted due to a conflict with a reservation held on the accessed namespace")]
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
    #[error("Error code not recognized")]
    #[fallback]
    Unknown,
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

impl core::fmt::Display for StatusCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StatusCode::Generic(code) => write!(f, "{code} (type=0h, code={:02x}h)", *code as u8),
            StatusCode::CommandSpecific(code) => write!(f, "Command specific error (type=1h, code={:02x}h)", code),
            StatusCode::MediaIntegrity(code) => write!(f, "Media integrity error (type=2h, code={:02x}h)", code),
            StatusCode::PathRelated(code) => write!(f, "Path related error (type=3h, code={:02x}h)", code),
            StatusCode::Unknown(code) => write!(f, "Unknown error (type=4h-7h, code={:02x}h)", code),
            StatusCode::InvalidStatusField => write!(f, "Invalid status field"),
        }
    }
}

impl TryFrom<u32> for StatusCode {
    type Error = u32;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        let bytes = value.to_le_bytes();
        let code = bytes[0]; // Status code is exactly 8 bits.
        let status_code_type = bytes[1] & 0b111; // Status code type is exactly 3 bits.
        let generic = GenericStatusCode::try_from(code).unwrap_or(GenericStatusCode::Unknown);
        let status_code = match status_code_type {
            _ if status_code_type == StatusCodeType::Generic as u8 => Self::Generic(generic),
            _ if status_code_type == StatusCodeType::CommandSpecific as u8 => Self::CommandSpecific(code),
            _ if status_code_type == StatusCodeType::MediaIntegrity as u8 => Self::MediaIntegrity(code),
            _ if status_code_type == StatusCodeType::PathRelated as u8 => Self::PathRelated(code),
            _ => Self::Unknown(code),
        };
        Ok(status_code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_code_from_integer_generic() {
        let encoded = 0b1_000_00000001; // First bit should be ignored.
        let status = StatusCode::try_from(encoded).unwrap();
        assert_eq!(status, StatusCode::Generic(GenericStatusCode::InvalidCommandOpcode));
    }

    #[test]
    fn status_code_from_integer_cmd_specific() {
        let encoded = 0b1_001_00000001;
        let status = StatusCode::try_from(encoded).unwrap();
        assert_eq!(status, StatusCode::CommandSpecific(1));
    }
}
