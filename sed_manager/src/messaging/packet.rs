use std::io::Seek;

use crate::serialization::{
    with_len::WithLen, without_len::WithoutLen, Deserialize, Error as SerializeError, Serialize,
};

/// The transfer length for IF-RECV for HANDLE_COM_ID_REQUESTs that fits the
/// response for NO_RESPONSE_AVAILABLE, VERIFY_COM_ID_VALID, and STACK_RESET
/// commands.
/// The device pads the response with zeros if the actual response is shorter.
pub const HANDLE_COM_ID_RESPONSE_LEN: usize = 46;
pub const COM_PACKET_HEADER_LEN: usize = 20;
pub const PACKET_HEADER_LEN: usize = 24;
pub const SUB_PACKET_HEADER_LEN: usize = 12;
pub const CREDIT_CONTROL_SUB_PACKET_LEN: usize = 16;
pub const HANDLE_COM_ID_PROTOCOL: u8 = 0x02;
pub const TOKENIZED_PROTOCOL: u8 = 0x01;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum SubPacketKind {
    Data = 0x0000,
    CreditControl = 0x8001,
}

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
#[repr(u16)]
pub enum AckType {
    ACK = 0x0001,
    NAK = 0x0002,
    None = 0x0000,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum FeatureCode {
    TPer = 0x0001,
    Locking = 0x0002,
    Enterprise = 0x0100,
    OpalV1 = 0x0200,
    OpalV2 = 0x0203,
    Opalite = 0x0301,
    PyriteV1 = 0x0302,
    PyriteV2 = 0x0303,
    Ruby = 0x0304,
    KeyPerIO = 0x0305,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum OwnerPasswordState {
    SameAsMSID = 0x00,
    VendorSpecified = 0xFF,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ComIdRequestCode {
    VerifyComIdValid = 1,
    StackReset = 2,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct SubPacket {
    #[layout(offset = 6)]
    pub kind: SubPacketKind,
    #[layout(offset = 8, round = 4)]
    pub payload: WithLen<u8, u32>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct Packet {
    pub tper_session_number: u32,
    pub host_session_number: u32,
    pub sequence_number: u32,
    #[layout(offset = 14)]
    pub ack_type: AckType,
    pub acknowledgement: u32,
    pub payload: WithLen<SubPacket, u32>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ComPacket {
    #[layout(offset = 4)]
    pub com_id: u16,
    pub com_id_ext: u16,
    pub outstanding_data: u32,
    pub min_transfer: u32,
    pub payload: WithLen<Packet, u32>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HandleComIdRequest {
    pub com_id: u16,
    pub com_id_ext: u16,
    pub request_code: ComIdRequestCode,
}

impl HandleComIdRequest {
    pub fn verify_com_id_valid(com_id: u16, com_id_ext: u16) -> HandleComIdRequest {
        HandleComIdRequest { com_id, com_id_ext, request_code: ComIdRequestCode::VerifyComIdValid }
    }
    pub fn stack_reset(com_id: u16, com_id_ext: u16) -> HandleComIdRequest {
        HandleComIdRequest { com_id, com_id_ext, request_code: ComIdRequestCode::StackReset }
    }
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

#[derive(Serialize, Deserialize, Clone)]
pub struct DiscoveryHeader {
    pub length_of_data: u32,
    pub major_version: u16,
    pub minor_version: u16,
    #[layout(offset = 16)]
    pub vendor_unique: [u8; 32],
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Discovery {
    pub descs: WithLen<FeatureDescriptor, DiscoveryHeader>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FeatureDescriptorHeader {
    pub feature_code: FeatureCode,
    #[layout(offset = 2, bits = 4..=7)]
    pub version: u8,
    #[layout(offset = 3)]
    pub length: u8,
}

#[derive(Serialize, Deserialize, Clone)]
#[layout(round = 12)]
pub struct TPerFeatureDescriptor {
    #[layout(offset = 0, bits = 0..=0)]
    pub sync_supported: bool,
    #[layout(offset = 0, bits = 1..=1)]
    pub async_supported: bool,
    #[layout(offset = 0, bits = 2..=2)]
    pub ack_nak_supported: bool,
    #[layout(offset = 0, bits = 3..=3)]
    pub buffer_mgmt_supported: bool,
    #[layout(offset = 0, bits = 4..=4)]
    pub streaming_supported: bool,
    #[layout(offset = 0, bits = 6..=6)]
    pub com_id_mgmt_supported: bool,
}

#[derive(Serialize, Deserialize, Clone)]
#[layout(round = 12)]
pub struct LockingFeatureDescriptor {
    #[layout(offset = 0, bits = 0..=0)]
    pub locking_supported: bool,
    #[layout(offset = 0, bits = 1..=1)]
    pub locking_enabled: bool,
    #[layout(offset = 0, bits = 2..=2)]
    pub locked: bool,
    #[layout(offset = 0, bits = 3..=3)]
    pub media_encryption: bool,
    #[layout(offset = 0, bits = 4..=4)]
    pub mbr_enabled: bool,
    #[layout(offset = 0, bits = 5..=5)]
    pub mbr_done: bool,
    #[layout(offset = 0, bits = 6..=6)]
    pub mbr_shadowing_not_supported: bool,
    #[layout(offset = 0, bits = 7..=7)]
    pub hw_reset_supported: bool,
}

#[derive(Serialize, Deserialize, Clone)]
#[layout(round = 16)]
pub struct OpalV2FeatureDescriptor {
    pub base_com_id: u16,
    pub num_com_ids: u16,
    #[layout(offset = 5)]
    pub num_locking_admins_supported: u16,
    pub num_locking_users_supported: u16,
    pub initial_owner_pw: OwnerPasswordState,
    pub reverted_owner_pw: OwnerPasswordState,
}

#[derive(Clone)]
pub enum FeatureDescriptor {
    TPer(TPerFeatureDescriptor),
    Locking(LockingFeatureDescriptor),
    Enterprise,
    OpalV1,
    OpalV2(OpalV2FeatureDescriptor),
    Opalite,
    PyriteV1,
    PyriteV2,
    Ruby,
    KeyPerIO,
}

impl From<&FeatureDescriptor> for FeatureCode {
    fn from(value: &FeatureDescriptor) -> Self {
        match value {
            FeatureDescriptor::TPer(_) => FeatureCode::TPer,
            FeatureDescriptor::Locking(_) => FeatureCode::Locking,
            FeatureDescriptor::Enterprise => FeatureCode::Enterprise,
            FeatureDescriptor::OpalV1 => FeatureCode::OpalV1,
            FeatureDescriptor::OpalV2(_) => FeatureCode::OpalV2,
            FeatureDescriptor::Opalite => FeatureCode::Opalite,
            FeatureDescriptor::PyriteV1 => FeatureCode::PyriteV1,
            FeatureDescriptor::PyriteV2 => FeatureCode::PyriteV2,
            FeatureDescriptor::Ruby => FeatureCode::Ruby,
            FeatureDescriptor::KeyPerIO => FeatureCode::KeyPerIO,
        }
    }
}

impl Serialize<u8> for FeatureDescriptor {
    type Error = SerializeError;
    fn serialize(&self, stream: &mut crate::serialization::OutputStream<u8>) -> Result<(), Self::Error> {
        let start_pos = stream.stream_position().unwrap();
        let mut header = FeatureDescriptorHeader { feature_code: self.into(), version: 1, length: 0 };
        let descriptor_pos = stream.stream_position().unwrap();
        header.serialize(stream)?;
        match self {
            FeatureDescriptor::TPer(descriptor) => descriptor.serialize(stream),
            FeatureDescriptor::Locking(descriptor) => descriptor.serialize(stream),
            FeatureDescriptor::OpalV2(descriptor) => descriptor.serialize(stream),
            _ => Ok(()),
        }?;
        let end_pos = stream.stream_position().unwrap();
        header.length = (end_pos - descriptor_pos) as u8;
        stream.seek(std::io::SeekFrom::Start(start_pos)).unwrap();
        header.serialize(stream)?;
        stream.seek(std::io::SeekFrom::Start(end_pos)).unwrap();
        Ok(())
    }
}

impl Deserialize<u8> for FeatureDescriptor {
    type Error = SerializeError;
    fn deserialize(stream: &mut crate::serialization::InputStream<u8>) -> Result<Self, Self::Error> {
        let header = FeatureDescriptorHeader::deserialize(stream)?;
        match header.feature_code {
            FeatureCode::TPer => Ok(FeatureDescriptor::TPer(TPerFeatureDescriptor::deserialize(stream)?)),
            FeatureCode::Locking => Ok(FeatureDescriptor::Locking(LockingFeatureDescriptor::deserialize(stream)?)),
            FeatureCode::Enterprise => Ok(FeatureDescriptor::Enterprise),
            FeatureCode::OpalV1 => Ok(FeatureDescriptor::OpalV1),
            FeatureCode::OpalV2 => Ok(FeatureDescriptor::OpalV2(OpalV2FeatureDescriptor::deserialize(stream)?)),
            FeatureCode::Opalite => Ok(FeatureDescriptor::Opalite),
            FeatureCode::PyriteV1 => Ok(FeatureDescriptor::PyriteV1),
            FeatureCode::PyriteV2 => Ok(FeatureDescriptor::PyriteV2),
            FeatureCode::Ruby => Ok(FeatureDescriptor::Ruby),
            FeatureCode::KeyPerIO => Ok(FeatureDescriptor::KeyPerIO),
        }
    }
}

impl TryFrom<DiscoveryHeader> for usize {
    type Error = <usize as TryFrom<u32>>::Error;
    fn try_from(value: DiscoveryHeader) -> Result<Self, Self::Error> {
        Self::try_from(value.length_of_data)
    }
}

impl TryFrom<usize> for DiscoveryHeader {
    type Error = <u32 as TryFrom<usize>>::Error;
    fn try_from(value: usize) -> Result<DiscoveryHeader, Self::Error> {
        let length_of_data: u32 = value.try_into()?;
        Ok(DiscoveryHeader {
            length_of_data: length_of_data,
            major_version: 1,
            minor_version: 0,
            vendor_unique: [0; 32],
        })
    }
}

impl Discovery {
    pub fn new(descs: Vec<FeatureDescriptor>) -> Discovery {
        Discovery { descs: WithLen::new(descs) }
    }
    pub fn get(&self, feature_code: FeatureCode) -> Option<&FeatureDescriptor> {
        self.descs.iter().find(|feature_desc| -> bool { feature_code == FeatureCode::from(*feature_desc) })
    }
}

impl Default for Packet {
    fn default() -> Self {
        Self {
            tper_session_number: 0,
            host_session_number: 0,
            sequence_number: 0,
            ack_type: AckType::None,
            acknowledgement: 0,
            payload: WithLen::new(vec![]),
        }
    }
}

impl Packet {
    pub fn has_ack(&self) -> bool {
        self.ack_type != AckType::None
    }

    pub fn has_payload(&self) -> bool {
        !self.payload.is_empty()
    }

    pub fn is_empty(&self) -> bool {
        !self.has_ack() && !self.has_payload()
    }

    pub fn credit(&self) -> u32 {
        let credit = self
            .payload
            .iter()
            .filter(|s| s.kind == SubPacketKind::Data)
            .map(|s| s.payload.len())
            .reduce(|a, b| a + b);
        credit.unwrap_or(0) as u32
    }
}
