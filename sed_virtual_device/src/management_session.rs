//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU32, Ordering};

use sed_packet::packet::{Packet, SubPacket, SubPacketKind};
use sed_packet::session_id::SessionId;
use sed_packet::token_stream::ToTokens;
use sed_spec::methods::{
    ExtractResult, MethodCall, MethodParam, MethodStatus, MgmtMethodCall, MgmtMethodCallParams, Properties,
    PropertiesMethod, StartSession, SyncSession, extract_method,
};
use sed_spec::objects::CPinRef;
use sed_spec::preconfig::core::shared::authority::ANYBODY;
use sed_spec::preconfig::core::shared::invoking_id::SESSION_MANAGER;
use sed_spec::types::LifeCycleState;

use crate::internal_error::Expect;
use crate::session::Session;
use crate::tper::Tper;

#[derive(Debug)]
pub struct ManagementSession {
    next_tsn: AtomicU32,
    outbound_properties: Properties,
    capabilities: Properties,
    recv_buffer: VecDeque<u8>,
}

impl ManagementSession {
    pub fn new(capabilities: Properties) -> Self {
        Self {
            next_tsn: 1000.into(),
            outbound_properties: Properties::INITIAL_OPAL_FAMILY,
            capabilities,
            recv_buffer: VecDeque::new(),
        }
    }

    pub fn next_tsn(&self) -> u32 {
        self.next_tsn.fetch_add(1, Ordering::Relaxed)
    }

    #[must_use]
    pub fn dispatch(&mut self, tper: &Tper, sessions: &mut HashMap<SessionId, Session>, packet: Packet) -> Vec<Packet> {
        // The TPer SHALL discard and ignore the packets that exceed the maximum
        // gross packet size (Core 5.2.2.4.1.2).
        if packet.gross_size() > self.capabilities.max_gross_packet_size.get() {
            return vec![];
        }
        // The TPer SHALL discard and ignore the packets that exceed the maximum
        // number of sub-packets (Core 5.2.2.4.1.1).
        if packet.payload.len() > self.capabilities.max_subpackets.get() {
            return vec![];
        }

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
                ExtractResult::EndOfSession => self.reset(),
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
                    sessions
                        .insert(
                            session_id,
                            Session::new(tper, session_id, params.spid, self.capabilities.clone(), authority_uid)?
                        )
                        .is_none(),
                    "the same TSN was erronously assigned to multiple sessions"
                );

                Ok(tsn)
            }
            None => Err(MethodStatus::InvalidParameter),
        }
    }

    fn properties(&mut self, params: PropertiesMethod) -> MethodCall<PropertiesMethod> {
        let host_capabilities = match params {
            PropertiesMethod::Host { host_properties } => host_properties,
            PropertiesMethod::TPer { host_properties, .. } => host_properties,
        };

        let outbound_properties = host_capabilities.map(|host_capabilities| {
            let continued_tokens = host_capabilities.continued_tokens && self.capabilities.continued_tokens;
            host_capabilities.with_mutual_features(&self.capabilities).with_continued_tokens(continued_tokens)
        });

        if let Some(outbound_properties) = &outbound_properties {
            self.outbound_properties = outbound_properties.clone();
        }

        MethodCall {
            invoking_id: SESSION_MANAGER,
            method_id: PropertiesMethod::METHOD_ID,
            parameters: PropertiesMethod::TPer {
                properties: self.capabilities.clone(),
                host_properties: outbound_properties,
            },
            status: MethodStatus::Success,
        }
    }

    fn reset(&mut self) {
        self.recv_buffer.clear();
    }
}
