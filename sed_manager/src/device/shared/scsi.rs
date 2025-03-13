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

fn convert_buffer_len(num_bytes: u32, inc_512: bool) -> u32 {
    if inc_512 {
        assert_eq!(num_bytes % 512, 0);
        num_bytes / 512
    } else {
        num_bytes
    }
}

impl SecurityProtocolIn {
    pub fn new(security_protocol: u8, security_protocol_specific: u16, alloc_len_bytes: u32, inc_512: bool) -> Self {
        Self {
            opcode: Opcode::SecurityProtocolIn,
            security_protocol,
            security_protocol_specific,
            inc_512,
            allocation_length: convert_buffer_len(alloc_len_bytes, inc_512),
            control: 0,
        }
    }
}

impl SecurityProtocolOut {
    pub fn new(security_protocol: u8, security_protocol_specific: u16, trans_len_bytes: u32, inc_512: bool) -> Self {
        Self {
            opcode: Opcode::SecurityProtocolOut,
            security_protocol,
            security_protocol_specific,
            inc_512,
            transfer_length: convert_buffer_len(trans_len_bytes, inc_512),
            control: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn security_protocol_out_new_bytes() {
        let value = SecurityProtocolOut::new(0, 0, 235, false);
        assert_eq!(value.transfer_length, 235);
        assert_eq!(value.inc_512, false);
    }

    #[test]
    fn security_protocol_out_new_512_ok() {
        let value = SecurityProtocolOut::new(0, 0, 512, true);
        assert_eq!(value.transfer_length, 1);
        assert_eq!(value.inc_512, true);
    }

    #[test]
    #[should_panic]
    fn security_protocol_out_new_512_err() {
        let _ = SecurityProtocolOut::new(0, 0, 235, true);
    }

    #[test]
    fn security_protocol_in_new_bytes() {
        let value = SecurityProtocolIn::new(0, 0, 235, false);
        assert_eq!(value.allocation_length, 235);
        assert_eq!(value.inc_512, false);
    }

    #[test]
    fn security_protocol_in_new_512_ok() {
        let value = SecurityProtocolIn::new(0, 0, 512, true);
        assert_eq!(value.allocation_length, 1);
        assert_eq!(value.inc_512, true);
    }

    #[test]
    #[should_panic]
    fn security_protocol_in_new_512_err() {
        let _ = SecurityProtocolIn::new(0, 0, 235, true);
    }
}
