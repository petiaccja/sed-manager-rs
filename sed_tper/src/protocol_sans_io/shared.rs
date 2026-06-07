use std::{cmp::min, marker::PhantomData, time::Instant};

use sed_packet::{
    packet::{Packet, SubPacket, SubPacketKind},
    session_id::SessionId,
    token::{Command, ToTokens as _},
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

pub fn eos() -> Vec<u8> {
    Command::EndOfSession.to_tokens().expect("can not serialize EOS command")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    None,
    Send { protocol: u8, data: Vec<u8> },
    Recv { protocol: u8, transfer_len: usize },
    Sleep { until: Instant },
}

#[cfg(test)]
pub mod tests {
    use sed_packet::{Bytes, session_id::SessionId, token::ToTokens as _};
    use sed_spec::{
        methods::{
            CloseSession, MethodCall, MethodParam, MethodResult, MethodStatus, Random, RandomResult,
            SessionMethodParam as _, StartSession, SyncSession,
        },
        preconfig::{
            core::shared::invoking_id::{SESSION_MANAGER, THIS_SP},
            opal_2::admin::sp,
        },
    };

    pub fn start_session_call(session_id: SessionId) -> Vec<u8> {
        MethodCall {
            invoking_id: SESSION_MANAGER,
            method_id: StartSession::METHOD_ID,
            parameters: StartSession {
                host_session_id: session_id.hsn,
                spid: sp::ADMIN,
                write: true,
                host_challenge: None,
                host_exchange_authority: None,
                host_exchange_cert: None,
                host_signing_authority: None,
                host_signing_cert: None,
                session_timeout: None,
                trans_timeout: None,
                initial_credit: None,
                signed_hash: None,
            },
            status: MethodStatus::Success,
        }
        .to_tokens()
        .unwrap()
    }

    pub fn sync_session_call(session_id: SessionId, status: MethodStatus) -> Vec<u8> {
        MethodCall {
            invoking_id: SESSION_MANAGER,
            method_id: SyncSession::METHOD_ID,
            parameters: SyncSession {
                host_session_id: session_id.hsn,
                sp_session_id: session_id.tsn,
                sp_challenge: None,
                sp_exchange_cert: None,
                sp_signing_cert: None,
                trans_timeout: None,
                initial_credit: None,
                signed_hash: None,
            },
            status: status,
        }
        .to_tokens()
        .unwrap()
    }

    pub fn close_session_call(session_id: SessionId) -> Vec<u8> {
        MethodCall {
            invoking_id: SESSION_MANAGER,
            method_id: CloseSession::METHOD_ID,
            parameters: CloseSession { remote_session_number: session_id.tsn, local_session_number: session_id.hsn },
            status: MethodStatus::Success,
        }
        .to_tokens()
        .unwrap()
    }

    pub fn method_call() -> Vec<u8> {
        Random { count: 4, buffer_out: None }.to_call(THIS_SP).to_tokens().unwrap()
    }

    pub fn method_response() -> Vec<u8> {
        MethodResult(Ok(RandomResult { result: Bytes(vec![1, 2, 3]) })).to_tokens().unwrap()
    }
}
