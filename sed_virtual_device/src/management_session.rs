use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use sed_packet::packet::{Packet, SubPacket, SubPacketKind};
use sed_packet::session_id::SessionId;
use sed_packet::token::ToTokens;
use sed_spec::methods::{
    ExtractResult, MethodCall, MethodParam, MethodStatus, MgmtMethodCall, MgmtMethodCallParams, Properties,
    PropertiesMethod, StartSession, SyncSession, extract_method,
};
use sed_spec::objects::CPinRef;
use sed_spec::preconfig::core::shared::authority::ANYBODY;
use sed_spec::preconfig::core::shared::invoking_id::SESSION_MANAGER;

use crate::session::Session;
use crate::tper::TPer;

pub const CAPABILITIES: Properties = Properties {
    max_methods: usize::MAX,
    max_subpackets: usize::MAX,
    max_packets: usize::MAX,
    max_gross_packet_size: 65536,
    max_gross_compacket_size: 65536,
    max_gross_compacket_response_size: 65536,
    max_ind_token_size: 65480,
    max_agg_token_size: 65480,
    continued_tokens: false,
    seq_numbers: false,
    ack_nak: false,
    asynchronous: true,
    buffer_mgmt: false,
    max_retries: 3,
    trans_timeout: Duration::from_secs(10),
    def_trans_timeout: Duration::from_secs(10),
};

#[derive(Debug)]
pub struct ManagementSession {
    next_tsn: AtomicU32,
    properties: Properties,
    recv_buffer: VecDeque<u8>,
}

impl ManagementSession {
    pub fn new() -> Self {
        Self { next_tsn: 1000.into(), properties: Properties::ASSUMED, recv_buffer: VecDeque::new() }
    }

    pub fn next_tsn(&self) -> u32 {
        self.next_tsn.fetch_add(1, Ordering::Relaxed)
    }

    #[must_use]
    pub fn dispatch(&mut self, tper: &TPer, sessions: &mut HashMap<SessionId, Session>, packet: Packet) -> Vec<Packet> {
        let data_sub_packets = packet.payload.iter().filter(|sub_packet| sub_packet.kind == SubPacketKind::Data);
        for sub_packet in data_sub_packets {
            self.recv_buffer.extend(sub_packet.payload.iter());
        }

        let mut response = Vec::new();

        loop {
            match extract_method(&mut self.recv_buffer) {
                ExtractResult::Ok { value, .. } => {
                    let packet = self.call(tper, sessions, value);
                    response.extend(packet);
                }
                ExtractResult::EndOfStream => self.reset(),
                ExtractResult::NeedMoreTokens => break,
                ExtractResult::InvalidTokens(_) => self.reset(),
            }
        }
        response
    }

    fn call(
        &mut self,
        tper: &TPer,
        sessions: &mut HashMap<SessionId, Session>,
        call: MgmtMethodCall,
    ) -> Option<Packet> {
        use MgmtMethodCallParams::*;

        let result_tokens = match call.params {
            StartSession(params) => Some(self.start_session(tper, sessions, params).to_tokens()),
            SyncSession(_) => None,  // Unexpected method call are ignored.
            CloseSession(_) => None, // Unexpected method call are ignored.
            Properties(params) => Some(self.properties(params).to_tokens()),
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

    fn start_session(
        &mut self,
        tper: &TPer,
        sessions: &mut HashMap<SessionId, Session>,
        params: StartSession,
    ) -> MethodCall<SyncSession> {
        let outcome = self.start_session_impl(tper, &params);
        let authority = params.host_signing_authority.unwrap_or(ANYBODY);
        if let Ok(tsn) = outcome {
            let session_id = SessionId { hsn: params.host_session_id, tsn };
            assert!(
                sessions.insert(session_id, Session::new(session_id, params.spid, authority)).is_none(),
                "TSN reused"
            );
        }
        match outcome {
            Ok(tsn) => MethodCall {
                invoking_id: SESSION_MANAGER,
                method_id: SyncSession::METHOD_ID,
                parameters: SyncSession {
                    host_session_id: params.host_session_id,
                    sp_session_id: tsn,
                    sp_challenge: None,
                    sp_exchange_cert: None,
                    sp_signing_cert: None,
                    trans_timeout: None,
                    initial_credit: None,
                    signed_hash: None,
                },
                status: MethodStatus::Success,
            },
            Err(status) => MethodCall {
                invoking_id: SESSION_MANAGER,
                method_id: SyncSession::METHOD_ID,
                parameters: SyncSession {
                    host_session_id: params.host_session_id,
                    sp_session_id: 0,
                    sp_challenge: None,
                    sp_exchange_cert: None,
                    sp_signing_cert: None,
                    trans_timeout: None,
                    initial_credit: None,
                    signed_hash: None,
                },
                status,
            },
        }
    }

    fn start_session_impl(&mut self, tper: &TPer, params: &StartSession) -> Result<u32, MethodStatus> {
        let (sp_uid, authority, password) = (params.spid, &params.host_signing_authority, &params.host_challenge);
        let authority = authority.unwrap_or(ANYBODY);

        match tper.sp(sp_uid) {
            Some(sp) => {
                let authorities = sp.authority();
                let authority = authorities.get(&authority).ok_or(MethodStatus::InvalidParameter)?;
                if let Some(credential) = authority.credential {
                    let credential: CPinRef = credential.try_into().expect("invalid credential in preconfig");
                    let c_pins = sp.c_pin();
                    let credential = c_pins.get(&credential).expect("credential missing from C_PIN table");
                    if &credential.pin != password {
                        return Err(MethodStatus::NotAuthorized);
                    }
                }
                let tsn = self.next_tsn.fetch_add(1, Ordering::Relaxed);
                Ok(tsn)
            }
            None => Err(MethodStatus::InvalidParameter),
        }
    }

    fn properties(&mut self, params: PropertiesMethod) -> MethodCall<PropertiesMethod> {
        let host_properties = match &params {
            PropertiesMethod::Host { host_properties } => host_properties.as_ref(),
            PropertiesMethod::TPer { host_properties, .. } => host_properties.as_ref(),
        };
        let common = Properties::common(host_properties.unwrap_or(&Properties::ASSUMED), &CAPABILITIES);
        self.properties = common.clone();
        MethodCall {
            invoking_id: SESSION_MANAGER,
            method_id: PropertiesMethod::METHOD_ID,
            parameters: PropertiesMethod::TPer { properties: CAPABILITIES, host_properties: Some(common) },
            status: MethodStatus::Success,
        }
    }

    fn reset(&mut self) {
        self.recv_buffer.clear();
    }
}
