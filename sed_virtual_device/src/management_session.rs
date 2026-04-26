use std::collections::{HashMap, VecDeque};

use sed_packet::packet::{Packet, SubPacket, SubPacketKind};
use sed_packet::session_id::SessionId;
use sed_packet::token::ToTokens;
use sed_spec::methods::{
    ExtractResult, MgmtMethodCall, MgmtMethodCallParams, PropertiesMethod, StartSession, SyncSession, extract_method,
};

use crate::session::Session;
use crate::tper::TPer;

#[derive(Debug)]
pub struct ManagementSession {
    recv_buffer: VecDeque<u8>,
}

impl ManagementSession {
    pub fn new() -> Self {
        Self { recv_buffer: VecDeque::new() }
    }

    pub fn dispatch(&mut self, tper: &TPer, sessions: &mut HashMap<SessionId, Session>, packet: Packet) -> Vec<Packet> {
        let data_sub_packets = packet.payload.iter().filter(|sub_packet| sub_packet.kind == SubPacketKind::Data);
        for sub_packet in data_sub_packets {
            self.recv_buffer.extend(sub_packet.payload.iter());
        }

        let mut response = Vec::new();

        loop {
            match extract_method(&mut self.recv_buffer) {
                ExtractResult::Ok { value, .. } => {
                    let packet = Self::call(tper, sessions, value);
                    response.extend(packet);
                }
                ExtractResult::EndOfStream => self.reset(),
                ExtractResult::NeedMoreTokens => break,
                ExtractResult::InvalidTokens(_) => self.reset(),
            }
        }
        response
    }

    fn call(tper: &TPer, sessions: &mut HashMap<SessionId, Session>, call: MgmtMethodCall) -> Option<Packet> {
        use MgmtMethodCallParams::*;

        let result_tokens = match call.params {
            StartSession(params) => Some(Self::start_session(tper, sessions, params).to_tokens()),
            SyncSession(_) => None,  // Unexpected method call are ignored.
            CloseSession(_) => None, // Unexpected method call are ignored.
            Properties(params) => Some(Self::properties(params).to_tokens()),
        };
        result_tokens.map(|result_tokens| Packet {
            payload: vec![SubPacket {
                kind: SubPacketKind::Data,
                length: std::marker::PhantomData,
                payload: result_tokens.expect("failed to serialize method result"),
            }],
            ..Default::default()
        })
    }

    fn start_session(tper: &TPer, sessions: &mut HashMap<SessionId, Session>, params: StartSession) -> SyncSession {
        todo!()
    }

    fn properties(params: PropertiesMethod) -> PropertiesMethod {
        todo!()
    }

    fn reset(&mut self) {
        self.recv_buffer.clear();
    }
}
