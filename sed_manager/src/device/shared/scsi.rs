use crate::serialization::Serialize;

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Opcode {
    SecurityProtocolOut = 0xB5,
    SecurityProtocolIn = 0xA2,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct SecurityProtocolIn {
    opcode: Opcode,
    security_protocol: u8,
    security_protocol_specific: u16,
    #[layout(offset = 4, bits = 0..=0)]
    inc_512: bool,
    #[layout(offset = 6)]
    allocation_length: u32,
    #[layout(offset = 11)]
    control: u8,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct SecurityProtocolOut {
    opcode: Opcode,
    security_protocol: u8,
    security_protocol_specific: u16,
    #[layout(offset = 4, bits = 0..=0)]
    inc_512: bool,
    #[layout(offset = 6)]
    transfer_length: u32,
    #[layout(offset = 11)]
    control: u8,
}

impl SecurityProtocolIn {
    pub fn new(security_protocol: u8, security_protocol_specific: u16, allocation_length: u32) -> Self {
        Self {
            opcode: Opcode::SecurityProtocolIn,
            security_protocol,
            security_protocol_specific,
            inc_512: false,
            allocation_length,
            control: 0,
        }
    }
}

impl SecurityProtocolOut {
    pub fn new(security_protocol: u8, security_protocol_specific: u16, transfer_length: u32) -> Self {
        Self {
            opcode: Opcode::SecurityProtocolOut,
            security_protocol,
            security_protocol_specific,
            inc_512: false,
            transfer_length,
            control: 0,
        }
    }
}
