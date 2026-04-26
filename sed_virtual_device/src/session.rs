use std::collections::VecDeque;

use sed_packet::Ignore;
use sed_packet::packet::{Packet, SubPacketKind};
use sed_spec::methods::MethodCall;

use crate::tper::TPer;

type AnyMethodCall = MethodCall<Vec<Ignore>>;

#[derive(Debug)]
pub enum Session {
    Open { recv_buffer: VecDeque<u8> },
    Closed,
}

impl Session {
    pub fn new() -> Self {
        Self::Open { recv_buffer: VecDeque::new() }
    }

    pub fn dispatch(&mut self, _tper: &mut TPer, packet: Packet) {
        match self {
            Session::Open { recv_buffer } => {
                let data_sub_packets =
                    packet.payload.iter().filter(|sub_packet| sub_packet.kind == SubPacketKind::Data);
                for sub_packet in data_sub_packets {
                    recv_buffer.extend(sub_packet.payload.iter());
                }
            }
            Session::Closed => (),
        }
    }
}
