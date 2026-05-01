use std::collections::{HashSet, VecDeque};

use crate::internal_error::Expect;
use sed_packet::packet::{Packet, SubPacket, SubPacketKind};
use sed_packet::session_id::SessionId;
use sed_packet::token::{Command, ToTokens};
use sed_packet::{Bytes, Uid};
use sed_spec::methods::{
    Activate, ActivateResult, Authenticate, AuthenticateResult, CloseSession, ExtractResult, MethodResult,
    MethodStatus, MgmtMethodCall, MgmtMethodCallParams, Random, RandomResult, SessionMethodCall,
    SessionMethodCallParams, SessionMethodParam as _, extract_method,
};
use sed_spec::objects::{AccessControlRef, AceExpr, AuthorityRef, MethodRef, SecurityProviderRef};
use sed_spec::preconfig::core::shared::invoking_id::THIS_SP;
use sed_spec::types::LifeCycleState;

use crate::tper::TPer;

#[derive(Debug)]
pub enum Session {
    Open {
        session_id: SessionId,
        sp: SecurityProviderRef,
        authenticated: HashSet<AuthorityRef>,
        recv_buffer: VecDeque<u8>,
    },
    Closed,
}

impl Session {
    pub fn new(
        tper: &TPer,
        session_id: SessionId,
        sp_uid: SecurityProviderRef,
        authority_uid: AuthorityRef,
    ) -> Result<Self, MethodStatus> {
        let sp = tper.sp(sp_uid).ok_or(MethodStatus::InvalidParameter)?;
        let authority = sp.authority().get(&authority_uid).ok_or(MethodStatus::InvalidParameter)?;
        let authenticated = std::iter::once(authority_uid).chain(authority.class).collect();
        Ok(Self::Open { session_id, sp: sp_uid, authenticated, recv_buffer: VecDeque::new() })
    }

    #[must_use]
    pub fn dispatch(&mut self, tper: &mut TPer, packet: Packet) -> Vec<Packet> {
        if let Session::Open { recv_buffer, .. } = self {
            let data_sub_packets = packet.payload.iter().filter(|sub_packet| sub_packet.kind == SubPacketKind::Data);
            for sub_packet in data_sub_packets {
                recv_buffer.extend(sub_packet.payload.iter());
            }
        };

        let mut extracted_methods = Vec::new();
        if let Self::Open { recv_buffer, .. } = self {
            loop {
                match extract_method::<SessionMethodCall>(recv_buffer) {
                    value @ ExtractResult::Ok { .. } => extracted_methods.push(value),
                    ExtractResult::NeedMoreTokens => break,
                    value => {
                        extracted_methods.push(value);
                        break;
                    }
                }
            }
        }

        extracted_methods
            .iter()
            .filter_map(|extract_result| match &extract_result {
                ExtractResult::Ok { value, .. } => self.call(tper, value),
                ExtractResult::EndOfStream => self.close(),
                ExtractResult::NeedMoreTokens => None,
                ExtractResult::InvalidTokens(_) => self.abort(),
            })
            .collect()
    }

    fn call(&mut self, tper: &mut TPer, call: &SessionMethodCall) -> Option<Packet> {
        use SessionMethodCallParams::*;

        if let Self::Open { session_id, .. } = self {
            let session_id = *session_id;

            let invoking_id = call.invoking_id;
            let result_tokens = match &call.params {
                Activate(params) => MethodResult(self.activate(tper, invoking_id, params)).to_tokens(),
                Authenticate(params) => MethodResult(self.authenticate(tper, invoking_id, params)).to_tokens(),
                GenKey(_params) => todo!(),
                Get(_params) => todo!(),
                GetAcl(_params) => todo!(),
                Next(_params) => todo!(),
                Random(params) => MethodResult(self.random(tper, invoking_id, params)).to_tokens(),
                Revert(_params) => todo!(),
                RevertSp(_params) => todo!(),
                SetAce(_params) => todo!(),
                SetAuthority(_params) => todo!(),
                SetBytes(_params) => todo!(),
                SetCPin(_params) => todo!(),
                SetKAes256(_params) => todo!(),
                SetLockingRange(_params) => todo!(),
                SetMbrControl(_params) => todo!(),
                SetSecurityProvider(_params) => todo!(),
                SetTableDesc(_params) => todo!(),
            };
            tracing::debug!(method_result = tracing::field::debug(&result_tokens), "response");
            Some(session_id.assign(Packet {
                payload: vec![SubPacket {
                    kind: SubPacketKind::Data,
                    length: std::marker::PhantomData,
                    payload: result_tokens.expect_serialize(),
                }],
                ..Default::default()
            }))
        } else {
            None
        }
    }

    fn activate(&self, tper: &mut TPer, invoking_id: Uid, params: &Activate) -> Result<ActivateResult, MethodStatus> {
        use sed_spec::preconfig::opal_2::admin;
        use sed_spec::preconfig::opal_2::locking;

        self.check_permission(tper, invoking_id, params.method_id().try_into().unwrap(), [0].into_iter())?;

        let sp_uid = SecurityProviderRef::try_from(invoking_id).map_err(|_| MethodStatus::InvalidParameter)?;
        let admin_sp = tper.admin_sp_mut();
        let sid_pin = admin_sp.c_pin.get(&admin::c_pin::SID).expect_object("C_PIN", "SID").pin.clone();
        let sp_info = admin_sp.sp.get_mut(&sp_uid).ok_or(MethodStatus::InvalidParameter)?;
        if sp_info.life_cycle_state == Some(LifeCycleState::ManufacturedInactive) {
            sp_info.life_cycle_state = Some(LifeCycleState::Manufactured);
            let sp = tper.sp_mut(sp_uid).expect_sp(sp_uid);
            let admin1_pin =
                sp.c_pin_mut().get_mut(&locking::c_pin::ADMIN.get(1).unwrap()).expect_object("C_PIN", "ADMIN1");
            admin1_pin.pin = sid_pin;
            Ok(ActivateResult)
        } else {
            Err(MethodStatus::SPDisabled)
        }
    }

    fn authenticate(
        &mut self,
        tper: &mut TPer,
        invoking_id: Uid,
        params: &Authenticate,
    ) -> Result<AuthenticateResult, MethodStatus> {
        self.check_permission(tper, invoking_id, params.method_id().try_into().unwrap(), [0].into_iter())?;

        let Self::Open { sp: sp_uid, authenticated, .. } = self else {
            return Err(MethodStatus::Fail);
        };

        let sp = tper.sp(*sp_uid).expect_sp(*sp_uid);
        let authority = sp.authority().get(&params.authority).ok_or(MethodStatus::InvalidParameter)?;
        if authority.is_class == Some(true) {
            return Err(MethodStatus::InvalidParameter);
        }
        if let Some(c_pin_uid) = authority.credential {
            let c_pin_uid = c_pin_uid.try_into().expect("internal error: non-PIN authentication");
            let c_pin = sp.c_pin().get(&c_pin_uid).expect_object("C_PIN", c_pin_uid);
            if c_pin.pin == params.proof {
                authenticated.insert(params.authority);
                if let Some(class) = authority.class {
                    authenticated.insert(class);
                }
                Ok(AuthenticateResult::Success(true))
            } else {
                Ok(AuthenticateResult::Success(false))
            }
        } else {
            authenticated.insert(params.authority);
            Ok(AuthenticateResult::Success(true))
        }
    }

    fn random(&self, tper: &TPer, invoking_id: Uid, params: &Random) -> Result<RandomResult, MethodStatus> {
        use rand::prelude::*;

        self.check_permission(tper, invoking_id, params.method_id().try_into().unwrap(), [0].into_iter())?;

        if invoking_id == THIS_SP {
            let mut rng = rand::rng();
            let mut bytes = Vec::new();
            bytes.resize_with(params.count as usize, || rng.random());
            Ok(RandomResult { result: Bytes(bytes) })
        } else {
            Err(MethodStatus::InvalidParameter)
        }
    }

    fn close(&mut self) -> Option<Packet> {
        if let Self::Open { session_id, .. } = self {
            let session_id = *session_id;
            *self = Self::Closed;

            let call = Command::EndOfSession;
            Some(session_id.assign(Packet {
                payload: vec![SubPacket {
                    kind: SubPacketKind::Data,
                    length: std::marker::PhantomData,
                    payload: call.to_tokens().expect_tokenize(),
                }],
                ..Default::default()
            }))
        } else {
            None
        }
    }

    fn abort(&mut self) -> Option<Packet> {
        if let Self::Open { session_id, .. } = self {
            let session_id = *session_id;
            *self = Self::Closed;

            let call = MgmtMethodCall {
                params: MgmtMethodCallParams::CloseSession(CloseSession {
                    remote_session_number: session_id.hsn,
                    local_session_number: session_id.tsn,
                }),
                status: MethodStatus::Success,
            };
            Some(SessionId::MANAGEMENT.assign(Packet {
                payload: vec![SubPacket {
                    kind: SubPacketKind::Data,
                    length: std::marker::PhantomData,
                    payload: call.to_tokens().expect_tokenize(),
                }],
                ..Default::default()
            }))
        } else {
            None
        }
    }

    fn check_permission(
        &self,
        tper: &TPer,
        invoking_id: Uid,
        method_id: MethodRef,
        mut columns: impl Iterator<Item = u16>,
    ) -> Result<(), MethodStatus> {
        let Self::Open { sp, authenticated, .. } = self else {
            return Err(MethodStatus::Fail);
        };
        let sp = tper.sp(*sp).ok_or(MethodStatus::InvalidParameter)?;
        let ac_table = sp.access_control();

        let mut permitted_columns = HashSet::new();
        // Check both the invoking ID and its containing table. ACLs for the
        // containing table apply to any object in the table.
        for invoking_id in std::iter::once(invoking_id).chain(invoking_id.containing_table()) {
            if let Some(access_control) = ac_table.get(&AccessControlRef { invoking_id, method_id }) {
                let ace_table = sp.ace();
                for ace_ref in &access_control.acl {
                    let ace = ace_table.get(ace_ref).expect_object("ACE", ace_ref);
                    let has_permission = ace
                        .boolean_expr
                        .as_ref()
                        .map(|expr| expr.eval(authenticated.iter().cloned()))
                        .flatten()
                        .unwrap_or(false);
                    if has_permission {
                        permitted_columns.extend(ace.columns.as_ref().unwrap_or(&HashSet::new()).iter().cloned());
                    }
                }
            }
        }

        if columns.all(|column| permitted_columns.contains(&column)) {
            Ok(())
        } else {
            Err(MethodStatus::NotAuthorized)
        }
    }
}
