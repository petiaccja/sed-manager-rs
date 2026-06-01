use std::collections::{HashMap, VecDeque};
use std::num::NonZero;
use std::sync::atomic::{AtomicU32, Ordering};

use sed_packet::packet::{
    COM_PACKET_HEADER_LEN, PACKET_HEADER_LEN, Packet, SUB_PACKET_HEADER_LEN, SubPacket, SubPacketKind,
};
use sed_packet::session_id::SessionId;
use sed_packet::token::ToTokens;
use sed_spec::methods::{
    ExtractResult, Limit, MethodCall, MethodParam, MethodStatus, MgmtMethodCall, MgmtMethodCallParams, OptionalLimit,
    Properties, PropertiesMethod, StartSession, SyncSession, extract_method,
};
use sed_spec::objects::CPinRef;
use sed_spec::preconfig::core::shared::authority::ANYBODY;
use sed_spec::preconfig::core::shared::invoking_id::SESSION_MANAGER;
use sed_spec::types::LifeCycleState;

use crate::internal_error::Expect;
use crate::session::Session;
use crate::tper::Tper;

/// Give it a realistic number based on what I've seen on real hardware.
const MAX_GROSS_COM_PACKET_SIZE: usize = 65536;

pub const CAPABILITIES: Properties = Properties {
    max_methods: Limit::Unlimited,
    max_subpackets: Limit::Unlimited,
    max_gross_packet_size: Limit::Limited(NonZero::new(MAX_GROSS_COM_PACKET_SIZE - COM_PACKET_HEADER_LEN).unwrap()),
    max_packets: Limit::Unlimited,
    max_gross_compacket_size: Limit::Limited(NonZero::new(MAX_GROSS_COM_PACKET_SIZE).unwrap()),
    max_gross_compacket_response_size: Limit::Limited(NonZero::new(MAX_GROSS_COM_PACKET_SIZE).unwrap()),
    max_sessions: Some(Limit::Unlimited),
    max_read_sessions: Some(Limit::Unlimited),
    max_ind_token_size: Limit::Limited(
        NonZero::new(MAX_GROSS_COM_PACKET_SIZE - COM_PACKET_HEADER_LEN - PACKET_HEADER_LEN - SUB_PACKET_HEADER_LEN)
            .unwrap(),
    ),
    max_agg_token_size: Limit::Limited(
        NonZero::new(MAX_GROSS_COM_PACKET_SIZE - COM_PACKET_HEADER_LEN - PACKET_HEADER_LEN - SUB_PACKET_HEADER_LEN)
            .unwrap(),
    ),
    max_authentications: Some(Limit::Unlimited),
    max_transaction_limit: Some(Limit::Unlimited),
    def_session_timeout: Some(Limit::Unlimited),
    max_session_timeout: Some(Limit::Unlimited),
    min_session_timeout: Some(OptionalLimit::Unsupported),
    def_trans_timeout: Some(Limit::Unlimited),
    max_trans_timeout: Some(Limit::Unlimited),
    min_trans_timeout: Some(OptionalLimit::Unsupported),
    max_com_id_time: Some(Limit::Unlimited),
    continued_tokens: false,
    seq_numbers: false,
    ack_nak: false,
    asynchronous: false,
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
    pub fn dispatch(&mut self, tper: &Tper, sessions: &mut HashMap<SessionId, Session>, packet: Packet) -> Vec<Packet> {
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
        tper: &Tper,
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
                payload: result_tokens.expect_serialize(),
            }],
            ..Default::default()
        })
    }

    fn start_session(
        &mut self,
        tper: &Tper,
        sessions: &mut HashMap<SessionId, Session>,
        params: StartSession,
    ) -> MethodCall<SyncSession> {
        let outcome = self.start_session_impl(tper, sessions, &params);
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

    fn start_session_impl(
        &mut self,
        tper: &Tper,
        sessions: &mut HashMap<SessionId, Session>,
        params: &StartSession,
    ) -> Result<u32, MethodStatus> {
        use LifeCycleState::*;

        let (sp_uid, authority, password) = (params.spid, &params.host_signing_authority, &params.host_challenge);

        let admin_sp = tper.admin_sp();
        let Some(sp_info) = admin_sp.sp.get(&sp_uid) else {
            return Err(MethodStatus::InvalidParameter);
        };

        match sp_info.life_cycle_state {
            Some(life_cycle_state) => match life_cycle_state {
                Issued | Manufactured => (),
                IssuedDisabled | ManufacturedDisabled => return Err(MethodStatus::SPDisabled),
                IssuedFrozen | ManufacturedFrozen => return Err(MethodStatus::SPFrozen),
                ManufacturedInactive | IssuedDisabledFrozen => return Err(MethodStatus::SPDisabled),
                ManufacturedDisabledFrozen => return Err(MethodStatus::SPDisabled),
                IssuedFailed | ManufacturedFailed => return Err(MethodStatus::Fail),
                Unknown(_) => return Err(MethodStatus::Fail),
            },
            None => unreachable!("life cycle state missing from SP preconfig"),
        };

        match tper.sp(sp_uid) {
            Some(sp) => {
                let authority_uid = authority.unwrap_or(ANYBODY);
                let authorities = sp.authority();
                let authority = authorities.get(&authority_uid).ok_or(MethodStatus::InvalidParameter)?;
                if let Some(credential) = authority.credential {
                    let credential: CPinRef = credential.try_into().expect("internal error: invalid credential ref");
                    let c_pins = sp.c_pin();
                    let credential = c_pins.get(&credential).expect_object("C_PIN", credential);
                    if &credential.pin != password {
                        return Err(MethodStatus::NotAuthorized);
                    }
                }
                let tsn = self.next_tsn.fetch_add(1, Ordering::Relaxed);
                let session_id = SessionId { hsn: params.host_session_id, tsn };
                assert!(
                    sessions.insert(session_id, Session::new(tper, session_id, params.spid, authority_uid)?).is_none(),
                    "the same TSN was erronously assigned to multiple sessions"
                );

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
