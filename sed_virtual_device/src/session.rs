use std::collections::VecDeque;

use sed_packet::packet::{Packet, SubPacket, SubPacketKind};
use sed_packet::session_id::SessionId;
use sed_packet::token::{Command, ToTokens};
use sed_packet::{Bytes, Uid};
use sed_spec::methods::{
    CloseSession, ExtractResult, MethodResult, MethodStatus, MgmtMethodCall, MgmtMethodCallParams, Random,
    RandomResult, SessionMethodCall, SessionMethodCallParams, extract_method,
};
use sed_spec::preconfig::core::shared::invoking_id::THIS_SP;

use crate::tper::TPer;

#[derive(Debug)]
pub enum Session {
    Open { session_id: SessionId, recv_buffer: VecDeque<u8> },
    Closed,
}

impl Session {
    pub fn new(session_id: SessionId) -> Self {
        Self::Open { recv_buffer: VecDeque::new(), session_id }
    }

    #[must_use]
    pub fn dispatch(&mut self, tper: &mut TPer, packet: Packet) -> Vec<Packet> {
        if let Session::Open { recv_buffer, .. } = self {
            let data_sub_packets = packet.payload.iter().filter(|sub_packet| sub_packet.kind == SubPacketKind::Data);
            for sub_packet in data_sub_packets {
                recv_buffer.extend(sub_packet.payload.iter());
            }
        };

        let mut response = Vec::new();

        *self = loop {
            *self = match core::mem::replace(self, Self::Closed) {
                Self::Open { session_id, mut recv_buffer } => match extract_method(&mut recv_buffer) {
                    ExtractResult::Ok { value, .. } => {
                        let packet = session_id.assign(Self::call(tper, value));
                        response.push(packet);
                        Self::Open { session_id, recv_buffer }
                    }
                    ExtractResult::EndOfStream => {
                        let packets = self.close();
                        response.extend(packets);
                        Self::Closed
                    }
                    ExtractResult::NeedMoreTokens => break Self::Open { session_id, recv_buffer },
                    ExtractResult::InvalidTokens(_) => {
                        let packets = self.abort();
                        response.extend(packets);
                        Self::Closed
                    }
                },
                Session::Closed => Session::Closed,
            }
        };
        response
    }

    fn call(tper: &mut TPer, call: SessionMethodCall) -> Packet {
        use SessionMethodCallParams::*;

        let invoking_id = call.invoking_id;
        let result_tokens = match call.params {
            Activate(params) => todo!(),
            Authenticate(params) => todo!(),
            Next(params) => todo!(),
            GetAcl(params) => todo!(),
            GenKey(params) => todo!(),
            Revert(params) => todo!(),
            RevertSp(params) => todo!(),
            Random(params) => MethodResult(Self::random(tper, invoking_id, params)).to_tokens(),
            Get(params) => todo!(),
            SetAce(params) => todo!(),
            SetAuthority(params) => todo!(),
            SetCPin(params) => todo!(),
            SetKAes256(params) => todo!(),
            SetLockingRange(params) => todo!(),
            SetMbrControl(params) => todo!(),
            SetSecurityProvider(params) => todo!(),
            SetTableDesc(params) => todo!(),
            SetBytes(params) => todo!(),
        };
        Packet {
            payload: vec![SubPacket {
                kind: SubPacketKind::Data,
                length: std::marker::PhantomData,
                payload: result_tokens.expect("failed to serialize method result"),
            }],
            ..Default::default()
        }
    }

    fn random(_tper: &mut TPer, invoking_id: Uid, params: Random) -> Result<RandomResult, MethodStatus> {
        use rand::prelude::*;

        if invoking_id == THIS_SP {
            let mut rng = rand::rng();
            let mut bytes = Vec::new();
            bytes.resize_with(params.count as usize, || rng.random());
            Ok(RandomResult { result: Bytes(bytes) })
        } else {
            Err(MethodStatus::InvalidParameter)
        }
    }

    fn close(&self) -> Option<Packet> {
        match self {
            Session::Open { session_id, .. } => {
                let call = Command::EndOfSession;
                let packet = session_id.assign(Packet {
                    payload: vec![SubPacket {
                        kind: SubPacketKind::Data,
                        length: std::marker::PhantomData,
                        payload: call.to_tokens().expect("method tokenization failed"),
                    }],
                    ..Default::default()
                });
                Some(packet)
            }
            Session::Closed => None,
        }
    }

    fn abort(&self) -> Option<Packet> {
        match self {
            Session::Open { session_id, .. } => {
                let call = MgmtMethodCall {
                    params: MgmtMethodCallParams::CloseSession(CloseSession {
                        remote_session_number: session_id.hsn,
                        local_session_number: session_id.tsn,
                    }),
                    status: MethodStatus::Success,
                };
                let packet = SessionId::MANAGEMENT.assign(Packet {
                    payload: vec![SubPacket {
                        kind: SubPacketKind::Data,
                        length: std::marker::PhantomData,
                        payload: call.to_tokens().expect("method tokenization failed"),
                    }],
                    ..Default::default()
                });
                Some(packet)
            }
            Session::Closed => None,
        }
    }
}
