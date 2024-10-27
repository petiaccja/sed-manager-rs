use crate::serialization::{with_len::WithLen, Deserialize, Serialize};

use super::value::Value;

/// The transfer length for IF-RECV for HANDLE_COM_ID_REQUESTs that fits the
/// response for NO_RESPONSE_AVAILABLE, VERIFY_COM_ID_VALID, and STACK_RESET
/// commands.
/// The device pads the response with zeros if the actual response is shorter.
pub const HANDLE_COM_ID_RESPONSE_LEN: usize = 46;

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(u16)]
pub enum SubPacketKind {
    Data = 0x0000,
    CreditControl = 0x8001,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(u32)]
pub enum ComIdState {
    Invalid = 0x00,
    Inactive = 0x01,
    Issued = 0x02,
    Associated = 0x03,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(u32)]
pub enum StackResetStatus {
    Success = 0,
    Failure = 1,
    Pending = 2,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(u32)]
pub enum ComIdRequestCode {
    VerifyComIdValid = 1,
    StackReset = 2,
}

#[derive(Serialize, Deserialize)]
pub struct SubPacket {
    #[layout(offset = 6)]
    pub kind: SubPacketKind,
    #[layout(offset = 8, round = 4)]
    pub payload: WithLen<Value, u32>,
}

#[derive(Serialize, Deserialize)]
pub struct Packet {
    pub tper_session_number: u32,
    pub host_session_number: u32,
    pub sequence_number: u32,
    #[layout(offset = 14)]
    pub ack_type: u16,
    pub acknowledgement: u32,
    pub payload: WithLen<SubPacket, u32>,
}

#[derive(Serialize, Deserialize)]
pub struct ComPacket {
    #[layout(offset = 4)]
    pub com_id: u16,
    pub com_id_ext: u16,
    pub outstanding_data: u32,
    pub min_transfer: u32,
    pub payload: WithLen<Packet, u32>,
}

#[derive(Serialize)]
pub struct HandleComIdRequest {
    pub com_id: u16,
    pub com_id_ext: u16,
    pub request_code: ComIdRequestCode,
}

impl HandleComIdRequest {
    pub fn verify_com_id_valid(com_id: u16, com_id_ext: u16) -> HandleComIdRequest {
        HandleComIdRequest { com_id: com_id, com_id_ext: com_id_ext, request_code: ComIdRequestCode::VerifyComIdValid }
    }
    pub fn stack_reset(com_id: u16, com_id_ext: u16) -> HandleComIdRequest {
        HandleComIdRequest { com_id: com_id, com_id_ext: com_id_ext, request_code: ComIdRequestCode::StackReset }
    }
}

/// The shared header for NO_RESPONSE_AVAILABLE, VERIFY_COM_ID_VALID, and
/// STACK_RESET responses. Deserialize this and immediately deserialize
/// the payloads if [`Self::available_data_len`] is not zero.
///
/// Example:
/// ```rust
/// use sed_manager::serialization::{InputStream, Deserialize};
/// use sed_manager::messaging::packet::{HandleComIdResponseHeader, VerifyComIdValidResponsePayload};
/// let mut stream = InputStream::<u8>::new(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00]);
/// let header = HandleComIdResponseHeader::deserialize(&mut stream).unwrap();
/// if header.available_data_len > 0 {
///     let payload = VerifyComIdValidResponsePayload::deserialize(&mut stream).unwrap();
/// }
/// ```
#[derive(Deserialize)]
pub struct HandleComIdResponseHeader {
    pub com_id: u16,
    pub com_id_ext: u16,
    pub request_code: ComIdRequestCode,
    #[layout(offset = 10)]
    pub available_data_len: u16,
}

/// See [`HandleComIdResponseHeader`].
#[derive(Deserialize)]
pub struct VerifyComIdValidResponsePayload {
    pub com_id_state: ComIdState,
}

/// See [`HandleComIdResponseHeader`].
#[derive(Deserialize)]
pub struct StackResetResponsePayload {
    pub stack_reset_status: StackResetStatus,
}
