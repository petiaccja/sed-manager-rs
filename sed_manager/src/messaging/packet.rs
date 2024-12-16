use crate::serialization::{with_len::WithLen, Deserialize, Serialize};

pub const COM_PACKET_HEADER_LEN: usize = 20;
pub const PACKET_HEADER_LEN: usize = 24;
pub const SUB_PACKET_HEADER_LEN: usize = 12;
pub const CREDIT_CONTROL_SUB_PACKET_LEN: usize = 16;
pub const PACKETIZED_PROTOCOL: u8 = 0x01;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum SubPacketKind {
    Data = 0x0000,
    CreditControl = 0x8001,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum AckType {
    ACK = 0x0001,
    NAK = 0x0002,
    None = 0x0000,
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
