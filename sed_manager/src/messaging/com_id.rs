use crate::serialization::{with_len::WithLen, Deserialize, InputStream, Serialize};

/// The transfer length for IF-RECV for HANDLE_COM_ID_REQUESTs that fits the
/// response for NO_RESPONSE_AVAILABLE, VERIFY_COM_ID_VALID, and STACK_RESET
/// commands. The device pads the response with zeros if the actual response is shorter.
pub const HANDLE_COM_ID_RESPONSE_LEN: usize = 46;
pub const HANDLE_COM_ID_PROTOCOL: u8 = 0x02;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ComIdState {
    Invalid = 0x00,
    Inactive = 0x01,
    Issued = 0x02,
    Associated = 0x03,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum StackResetStatus {
    Success = 0,
    Failure = 1,
    Pending = 2,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ComIdRequestCode {
    VerifyComIdValid = 1,
    StackReset = 2,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HandleComIdRequest {
    pub com_id: u16,
    pub com_id_ext: u16,
    pub request_code: ComIdRequestCode,
}

/// The shared structure for NO_RESPONSE_AVAILABLE, VERIFY_COM_ID_VALID, and
/// STACK_RESET responses. The payload field contains the payload for one of the above
/// messages.
#[derive(Serialize, Deserialize, Clone)]
pub struct HandleComIdResponse {
    pub com_id: u16,
    pub com_id_ext: u16,
    pub request_code: ComIdRequestCode,
    #[layout(offset = 10)]
    pub payload: WithLen<u8, u16>,
}

/// See [`HandleComIdResponse`].
#[derive(Serialize, Deserialize, Clone)]
pub struct VerifyComIdValidResponsePayload {
    pub com_id_state: ComIdState,
}

/// See [`HandleComIdResponse`].
#[derive(Serialize, Deserialize, Clone)]
pub struct StackResetResponsePayload {
    pub stack_reset_status: StackResetStatus,
}

impl HandleComIdRequest {
    pub fn verify_com_id_valid(com_id: u16, com_id_ext: u16) -> HandleComIdRequest {
        HandleComIdRequest { com_id, com_id_ext, request_code: ComIdRequestCode::VerifyComIdValid }
    }

    pub fn stack_reset(com_id: u16, com_id_ext: u16) -> HandleComIdRequest {
        HandleComIdRequest { com_id, com_id_ext, request_code: ComIdRequestCode::StackReset }
    }
}

impl HandleComIdResponse {
    pub fn verify_com_id_valid(&self) -> Option<VerifyComIdValidResponsePayload> {
        if self.request_code == ComIdRequestCode::VerifyComIdValid {
            let mut stream = InputStream::from(self.payload.clone().into_vec());
            VerifyComIdValidResponsePayload::deserialize(&mut stream).ok()
        } else {
            None
        }
    }

    pub fn stack_reset(&self) -> Option<StackResetResponsePayload> {
        if self.request_code == ComIdRequestCode::StackReset {
            let mut stream = InputStream::from(self.payload.clone().into_vec());
            StackResetResponsePayload::deserialize(&mut stream).ok()
        } else {
            None
        }
    }
}
