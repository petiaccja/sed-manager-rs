//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::ops::Deref;

use sorbit::{Deserialize, Serialize};

pub const COM_PACKET_HEADER_LEN: usize = 20;
pub const PACKET_HEADER_LEN: usize = 24;
pub const SUB_PACKET_HEADER_LEN: usize = 12;
pub const CREDIT_CONTROL_SUB_PACKET_LEN: usize = 16;
pub const PACKETIZED_PROTOCOL: u8 = 0x01;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
#[sorbit(byte_order=big_endian)]
pub enum SubPacketKind {
    Data = 0x0000,
    CreditControl = 0x8001,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
#[sorbit(byte_order=big_endian)]
pub enum AckType {
    ACK = 0x0001,
    NAK = 0x0002,
    None = 0x0000,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[sorbit(byte_order=big_endian)]
#[sorbit(round = 4)]
pub struct SubPacket {
    #[sorbit(offset = 6)]
    pub kind: SubPacketKind,
    #[sorbit(offset = 8, value = len(payload))] // Use len instead of byte_count as `Item=u8`.
    pub length: u32,
    pub payload: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[sorbit(byte_order=big_endian)]
pub struct Packet {
    pub tper_session_number: u32,
    pub host_session_number: u32,
    pub sequence_number: u32,
    #[sorbit(offset = 14)]
    pub ack_type: AckType,
    pub acknowledgement: u32,
    #[sorbit(value = byte_count(payload))]
    pub length: u32,
    pub payload: Vec<SubPacket>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[sorbit(byte_order=big_endian)]
pub struct ComPacket {
    #[sorbit(offset = 4)]
    pub com_id: u16,
    pub com_id_ext: u16,
    pub outstanding_data: u32,
    pub min_transfer: u32,
    #[sorbit(value = byte_count(payload))]
    pub length: u32,
    #[sorbit(multi_pass)]
    pub payload: Vec<Packet>,
}

impl Default for Packet {
    fn default() -> Self {
        Self {
            tper_session_number: 0,
            host_session_number: 0,
            sequence_number: 0,
            ack_type: AckType::None,
            acknowledgement: 0,
            length: 0,
            payload: Vec::new(),
        }
    }
}

impl Default for ComPacket {
    fn default() -> Self {
        Self { com_id: 0, com_id_ext: 0, outstanding_data: 0, min_transfer: 0, length: 0, payload: Vec::new() }
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

impl ComPacket {
    pub fn get_transfer_len(&self) -> u32 {
        let mut transfer_len = COM_PACKET_HEADER_LEN;
        for packet in self.payload.deref() {
            transfer_len += PACKET_HEADER_LEN;
            for sub_packet in packet.payload.deref() {
                transfer_len += SUB_PACKET_HEADER_LEN;
                transfer_len += (sub_packet.payload.len() + 3) / 4 * 4;
            }
        }
        transfer_len as u32
    }

    pub fn append(&mut self, mut other: Self) {
        self.payload.append(&mut other.payload);
        self.min_transfer = other.min_transfer;
        self.outstanding_data = other.outstanding_data;
    }
}

#[cfg(test)]
mod tests {
    use sorbit::ser_de::{FromBytes, ToBytes};

    use super::*;

    #[test]
    fn serialzie_sub_packet() {
        let bytes = [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Reserved.
            0x00, 0x00, // Kind.
            0x00, 0x00, 0x00, 0x06, // Length.
            0xCC, 0xCC, 0x0CC, 0xCC, 0xCC, 0xCC, // Payload.
            0x00, 0x00, // Padding.
        ];
        let value =
            SubPacket { kind: SubPacketKind::Data, length: 0x06, payload: vec![0xCC, 0xCC, 0x0CC, 0xCC, 0xCC, 0xCC] };
        assert_eq!(value.to_bytes().unwrap(), &bytes);
        assert_eq!(SubPacket::from_bytes(&bytes).unwrap(), value);
    }

    #[test]
    fn serialzie_packet() {
        let bytes = [
            // Packet header.
            0x01, 0x02, 0x03, 0x04, // TSN.
            0x05, 0x06, 0x07, 0x08, // HSN.
            0x09, 0x0A, 0x0B, 0x0C, // SN.
            0x00, 0x00, // Reserved.
            0x00, 0x01, // Ack type.
            0x0D, 0x0E, 0x0F, 0x01, // Acknowledgement.
            0x00, 0x00, 0x00, 0x14, // Length.
            // Sub packet.
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Reserved.
            0x00, 0x00, // Kind.
            0x00, 0x00, 0x00, 0x06, // Length.
            0xCC, 0xCC, 0x0CC, 0xCC, 0xCC, 0xCC, // Payload.
            0x00, 0x00, // Padding.
        ];
        let value = Packet {
            tper_session_number: 0x01020304,
            host_session_number: 0x05060708,
            sequence_number: 0x090A0B0C,
            ack_type: AckType::ACK,
            acknowledgement: 0x0D0E0F01,
            length: 0x14,
            payload: vec![SubPacket {
                kind: SubPacketKind::Data,
                length: 0x06,
                payload: vec![0xCC, 0xCC, 0x0CC, 0xCC, 0xCC, 0xCC],
            }],
        };

        assert_eq!(value.to_bytes().unwrap(), &bytes);
        assert_eq!(Packet::from_bytes(&bytes).unwrap(), value);
    }

    #[test]
    fn serialzie_com_packet() {
        let bytes = [
            // Com packet header.
            0x00, 0x00, 0x00, 0x00, // Reserved.
            0x12, 0x34, // ComID.
            0x00, 0x34, // ComID extension.
            0x00, 0x00, 0x00, 0x10, // Outstanding data.
            0x00, 0x00, 0x00, 0x10, // Min transfer.
            0x00, 0x00, 0x00, 0x2C, // Length.
            // Packet.
            0x01, 0x02, 0x03, 0x04, // TSN.
            0x05, 0x06, 0x07, 0x08, // HSN.
            0x09, 0x0A, 0x0B, 0x0C, // SN.
            0x00, 0x00, // Reserved.
            0x00, 0x01, // Ack type.
            0x0D, 0x0E, 0x0F, 0x01, // Acknowledgement.
            0x00, 0x00, 0x00, 0x14, // Length.
            // Sub packet.
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Reserved.
            0x00, 0x00, // Kind.
            0x00, 0x00, 0x00, 0x06, // Length.
            0xCC, 0xCC, 0x0CC, 0xCC, 0xCC, 0xCC, // Payload.
            0x00, 0x00, // Padding.
        ];
        let value = ComPacket {
            com_id: 0x1234,
            com_id_ext: 0x0034,
            outstanding_data: 0x10,
            min_transfer: 0x10,
            length: 0x2C,
            payload: vec![Packet {
                tper_session_number: 0x01020304,
                host_session_number: 0x05060708,
                sequence_number: 0x090A0B0C,
                ack_type: AckType::ACK,
                acknowledgement: 0x0D0E0F01,
                length: 0x14,
                payload: vec![SubPacket {
                    kind: SubPacketKind::Data,
                    length: 0x06,
                    payload: vec![0xCC, 0xCC, 0x0CC, 0xCC, 0xCC, 0xCC],
                }],
            }],
        };

        assert_eq!(value.to_bytes().unwrap(), &bytes);
        assert_eq!(ComPacket::from_bytes(&bytes).unwrap(), value);
    }
}
