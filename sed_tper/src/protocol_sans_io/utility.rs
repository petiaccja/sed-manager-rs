use std::{cmp::min, marker::PhantomData, time::Instant};

use sed_packet::{
    packet::{Packet, SubPacket, SubPacketKind},
    session_id::SessionId,
};

use crate::protocol_sans_io::sequence_number::SequenceNumber;

pub fn packetize_one(session_id: SessionId, sn: SequenceNumber, call: Vec<u8>) -> Packet {
    let sub_packet = SubPacket { kind: SubPacketKind::Data, length: PhantomData, payload: call };
    let packet = Packet { sequence_number: sn.0, payload: vec![sub_packet], ..Default::default() };
    session_id.assign(packet)
}

pub fn min_deadline(d1: Option<Instant>, d2: Option<Instant>) -> Option<Instant> {
    match (d1, d2) {
        (None, None) => None,
        (None, Some(d)) => Some(d),
        (Some(d), None) => Some(d),
        (Some(d1), Some(d2)) => Some(min(d1, d2)),
    }
}
