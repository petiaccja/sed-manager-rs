use super::value::Value;

#[repr(u16)]
pub enum SubPacketKind {
    Data = 0x0000,
    CreditControl = 0x8001,
}

pub struct SubPacket {
    pub kind: SubPacketKind,
    pub payload: Vec<Value>,
}

pub struct Packet {
    pub tper_session_number: u32,
    pub host_session_number: u32,
    pub sequence_number: u32,
    pub ack_type: u16,
    pub acknowledgement: u32,
    pub payload: Vec<SubPacket>,
}

pub struct ComPacket {
    pub com_id: u16,
    pub com_id_ext: u16,
    pub outstanding_data: u32,
    pub min_transfer: u32,
    pub payload: Vec<Packet>,
}
