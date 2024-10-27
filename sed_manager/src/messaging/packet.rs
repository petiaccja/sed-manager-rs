use crate::serialization::{with_len::WithLen, Deserialize, Serialize, SerializeError};

use super::value::Value;

#[derive(Clone, Copy, Debug)]
#[repr(u16)]
pub enum SubPacketKind {
    Data = 0x0000,
    CreditControl = 0x8001,
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

impl TryFrom<u16> for SubPacketKind {
    type Error = ();
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            _ if value == SubPacketKind::Data as u16 => Ok(SubPacketKind::Data),
            _ if value == SubPacketKind::CreditControl as u16 => Ok(SubPacketKind::CreditControl),
            _ => Err(()),
        }
    }
}

impl Serialize<SubPacketKind, u8> for SubPacketKind {
    type Error = SerializeError;
    fn serialize(&self, stream: &mut crate::serialization::OutputStream<u8>) -> Result<(), Self::Error> {
        (*self as u16).serialize(stream)
    }
}

impl Deserialize<SubPacketKind, u8> for SubPacketKind {
    type Error = SerializeError;
    fn deserialize(stream: &mut crate::serialization::InputStream<u8>) -> Result<SubPacketKind, Self::Error> {
        let value = u16::deserialize(stream)?;
        let Ok(x) = SubPacketKind::try_from(value) else {
            return Err(SerializeError::InvalidRepr);
        };
        Ok(x)
    }
}
