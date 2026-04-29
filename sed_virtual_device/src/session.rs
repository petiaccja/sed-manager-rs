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

        let next_state = match self {
            Session::Open { session_id, recv_buffer } => {
                let next_state = loop {
                    let extract_result = extract_method(recv_buffer);
                    match &extract_result {
                        ExtractResult::Ok { value, .. } => response.push(Self::call(tper, value)),
                        ExtractResult::EndOfStream => response.push(close(*session_id)),
                        ExtractResult::NeedMoreTokens => (),
                        ExtractResult::InvalidTokens(_) => response.push(abort(*session_id)),
                    };
                    match &extract_result {
                        ExtractResult::Ok { .. } => (),
                        ExtractResult::EndOfStream => break Some(Self::Closed),
                        ExtractResult::NeedMoreTokens => break None,
                        ExtractResult::InvalidTokens(_) => break Some(Self::Closed),
                    };
                };
                response.iter_mut().for_each(|packet| session_id.assign_in_place(packet));
                next_state
            }
            Session::Closed => None,
        };

        if let Some(next_state) = next_state {
            *self = next_state;
        }

        response
    }

    fn call(tper: &mut TPer, call: &SessionMethodCall) -> Packet {
        use SessionMethodCallParams::*;

        let invoking_id = call.invoking_id;
        let result_tokens = match &call.params {
            Activate(_params) => todo!(),
            Authenticate(_params) => todo!(),
            Next(_params) => todo!(),
            GetAcl(_params) => todo!(),
            GenKey(_params) => todo!(),
            Revert(_params) => todo!(),
            RevertSp(_params) => todo!(),
            Random(params) => MethodResult(Self::random(tper, invoking_id, params)).to_tokens(),
            Get(_params) => todo!(),
            SetAce(_params) => todo!(),
            SetAuthority(_params) => todo!(),
            SetCPin(_params) => todo!(),
            SetKAes256(_params) => todo!(),
            SetLockingRange(_params) => todo!(),
            SetMbrControl(_params) => todo!(),
            SetSecurityProvider(_params) => todo!(),
            SetTableDesc(_params) => todo!(),
            SetBytes(_params) => todo!(),
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

    fn random(_tper: &mut TPer, invoking_id: Uid, params: &Random) -> Result<RandomResult, MethodStatus> {
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
}

fn close(session_id: SessionId) -> Packet {
    let call = Command::EndOfSession;
    session_id.assign(Packet {
        payload: vec![SubPacket {
            kind: SubPacketKind::Data,
            length: std::marker::PhantomData,
            payload: call.to_tokens().expect("method tokenization failed"),
        }],
        ..Default::default()
    })
}

fn abort(session_id: SessionId) -> Packet {
    let call = MgmtMethodCall {
        params: MgmtMethodCallParams::CloseSession(CloseSession {
            remote_session_number: session_id.hsn,
            local_session_number: session_id.tsn,
        }),
        status: MethodStatus::Success,
    };
    SessionId::MANAGEMENT.assign(Packet {
        payload: vec![SubPacket {
            kind: SubPacketKind::Data,
            length: std::marker::PhantomData,
            payload: call.to_tokens().expect("method tokenization failed"),
        }],
        ..Default::default()
    })
}
