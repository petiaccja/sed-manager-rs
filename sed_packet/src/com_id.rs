//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sorbit::{Deserialize, Serialize};

/// The transfer length for IF-RECV for HANDLE_COM_ID_REQUESTs that fits the
/// response for NO_RESPONSE_AVAILABLE, VERIFY_COM_ID_VALID, and STACK_RESET
/// commands. The device pads the response with zeros if the actual response is shorter.
pub const COM_ID_RESPONSE_LEN: usize = 46;
pub const COM_ID_PROTOCOL: u8 = 0x02;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
#[sorbit(byte_order=big_endian)]
pub enum ComIdState {
    Invalid = 0x00,
    Inactive = 0x01,
    Issued = 0x02,
    Associated = 0x03,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
#[sorbit(byte_order=big_endian)]
pub enum StackResetStatus {
    Success = 0,
    Failure = 1,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
#[sorbit(byte_order=big_endian)]
pub enum ComIdRequestCode {
    NoResponseAvailable = 0,
    Verify = 1,
    StackReset = 2,
}

/// A HANDLE_COM_ID request sent to the TPer.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[sorbit(byte_order=big_endian)]
pub struct ComIdRequest {
    /// The ComID that is the subject of the request.
    pub com_id: u16,
    /// The extension of the ComID that is the subject of the request.
    pub com_id_ext: u16,
    /// The action the TPer should do.
    pub request_code: ComIdRequestCode,
}

/// The response sent by the TPer to a HANDLE_COM_ID request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[sorbit(byte_order=big_endian)]
pub struct ComIdResponse {
    /// The ComID that is the subject of the request.
    pub com_id: u16,
    /// The extension of the ComID that is the subject of the request.
    pub com_id_ext: u16,
    /// The response sent by the TPer.
    pub payload: ComIdResponsePayload,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[sorbit(byte_order=big_endian)]
#[repr(u32)]
pub enum ComIdResponsePayload {
    /// An indication that no response is available to ComID requests.
    ///
    /// You may see this when you haven't sent any requests or when a response
    /// has been fully read via IF-RECV and the device erased it.
    NoResponseAvailable {
        #[sorbit(offset = 2)]
        available_data_length: u16,
    } = ComIdRequestCode::NoResponseAvailable as u32,
    /// The response to the last request to verify a ComID.
    Verify {
        #[sorbit(offset = 2)]
        available_data_length: u16,
        com_id_state: ComIdState,
        time_of_allocation: Date,
        time_of_expiry: Date,
        time_since_reset: Date,
    } = ComIdRequestCode::Verify as u32,
    /// The outcome of the last stack reset request.
    ///
    /// There are three possible outcomes:
    /// - Success: the `available_data_length` is 4 and the status is [`Success`]
    /// - Failure: the `available_data_length` is 4 and the status is [`Failure`]
    /// - Pending: the `available_data_length` is 0 and the status is [`Success`] (== 0)
    ///
    /// [`Success`]: StackResetStatus::Success
    /// [`Failure`]: StackResetStatus::Failure
    StackReset {
        #[sorbit(offset = 2)]
        available_data_length: u16,
        status: StackResetStatus,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[sorbit(byte_order=big_endian, len=10)]
pub struct Date {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    millisecond: u16,
}

impl Date {
    pub fn unsupported() -> Self {
        Self { year: 0, month: 0, day: 0, hour: 0, minute: 0, second: 0, millisecond: 0 }
    }
}

impl ComIdRequest {
    pub fn verify_com_id_valid(com_id: u16, com_id_ext: u16) -> ComIdRequest {
        ComIdRequest { com_id, com_id_ext, request_code: ComIdRequestCode::Verify }
    }

    pub fn stack_reset(com_id: u16, com_id_ext: u16) -> ComIdRequest {
        ComIdRequest { com_id, com_id_ext, request_code: ComIdRequestCode::StackReset }
    }
}

impl Default for ComIdResponse {
    fn default() -> Self {
        Self {
            com_id: 0,
            com_id_ext: 0,
            payload: ComIdResponsePayload::NoResponseAvailable { available_data_length: 0 },
        }
    }
}

#[cfg(test)]
mod tests {
    use sorbit::ser_de::{FromBytes, ToBytes};

    use super::*;

    #[test]
    fn serialize_verify_comid_valid_request() {
        let bytes = [
            0x01, 0x02, // ComID.
            0x03, 0x04, // Extended ComID.
            0x00, 0x00, 0x00, 0x01, // Request code.
        ];
        let packet = ComIdRequest::verify_com_id_valid(0x0102, 0x0304);
        assert_eq!(packet.to_bytes().unwrap(), bytes);
        assert_eq!(ComIdRequest::from_bytes(&bytes).unwrap(), packet);
    }

    #[test]
    fn serialize_stack_reset_request() {
        let bytes = [
            0x01, 0x02, // ComID.
            0x03, 0x04, // Extended ComID.
            0x00, 0x00, 0x00, 0x02, // Request code.
        ];
        let packet = ComIdRequest::stack_reset(0x0102, 0x0304);
        assert_eq!(packet.to_bytes().unwrap(), bytes);
        assert_eq!(ComIdRequest::from_bytes(&bytes).unwrap(), packet);
    }

    #[test]
    fn serialize_no_response_available_response() {
        let bytes = [
            0x01, 0x02, // ComID.
            0x03, 0x04, // Extended ComID.
            0x00, 0x00, 0x00, 0x00, // Request code.
            0x00, 0x00, // Reserved.
            0x00, 0x00, // Available data length.
        ];
        let packet = ComIdResponse {
            com_id: 0x0102,
            com_id_ext: 0x0304,
            payload: ComIdResponsePayload::NoResponseAvailable { available_data_length: 0 },
        };
        assert_eq!(packet.to_bytes().unwrap(), bytes);
        assert_eq!(ComIdResponse::from_bytes(&bytes).unwrap(), packet);
    }

    #[test]
    fn serialize_verify_comid_valid_response() {
        let bytes = [
            0x01, 0x02, // ComID.
            0x03, 0x04, // Extended ComID.
            0x00, 0x00, 0x00, 0x01, // Request code.
            0x00, 0x00, // Reserved.
            0x00, 0x22, // Available data length.
            0x00, 0x00, 0x00, 0x02, // ComID state.
            0x07, 0xDC, 0x06, 0x12, 0x09, 0x20, 0x14, 0x01, 0x28, 0x00, // Alloc date.
            0x07, 0xDC, 0x06, 0x12, 0x09, 0x20, 0x14, 0x01, 0x28, 0x00, // Expiry date.
            0x07, 0xDC, 0x06, 0x12, 0x09, 0x20, 0x14, 0x01, 0x28, 0x00, // Reset date.
        ];
        let date = Date { year: 2012, month: 06, day: 18, hour: 9, minute: 32, second: 20, millisecond: 296 };
        let packet = ComIdResponse {
            com_id: 0x0102,
            com_id_ext: 0x0304,
            payload: ComIdResponsePayload::Verify {
                available_data_length: 0x22,
                com_id_state: ComIdState::Issued,
                time_of_allocation: date.clone(),
                time_of_expiry: date.clone(),
                time_since_reset: date,
            },
        };
        assert_eq!(packet.to_bytes().unwrap(), bytes);
        assert_eq!(ComIdResponse::from_bytes(&bytes).unwrap(), packet);
    }

    #[test]
    fn serialize_stack_reset_response() {
        let bytes = [
            0x01, 0x02, // ComID.
            0x03, 0x04, // Extended ComID.
            0x00, 0x00, 0x00, 0x02, // Request code.
            0x00, 0x00, // Reserved.
            0x00, 0x04, // Available data length.
            0x00, 0x00, 0x00, 0x01, // Failure.
        ];
        let packet = ComIdResponse {
            com_id: 0x0102,
            com_id_ext: 0x0304,
            payload: ComIdResponsePayload::StackReset { available_data_length: 4, status: StackResetStatus::Failure },
        };
        assert_eq!(packet.to_bytes().unwrap(), bytes);
        assert_eq!(ComIdResponse::from_bytes(&bytes).unwrap(), packet);
    }
}
