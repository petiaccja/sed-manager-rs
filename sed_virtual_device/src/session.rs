use std::collections::{HashSet, VecDeque};

use sed_packet::packet::{Packet, SubPacket, SubPacketKind};
use sed_packet::session_id::SessionId;
use sed_packet::token::{Command, ToTokens};
use sed_packet::{Bytes, Uid};
use sed_spec::methods::{
    CloseSession, ExtractResult, MethodParam, MethodResult, MethodStatus, MgmtMethodCall, MgmtMethodCallParams, Random,
    RandomResult, SessionMethodCall, SessionMethodCallParams, extract_method,
};
use sed_spec::objects::{AccessControlRef, AceExpr, AuthorityRef, MethodRef, SecurityProviderRef};
use sed_spec::preconfig::core::shared::invoking_id::THIS_SP;

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
    pub fn new(session_id: SessionId, sp: SecurityProviderRef, authority: AuthorityRef) -> Self {
        Self::Open { session_id, sp, authenticated: [authority].into(), recv_buffer: VecDeque::new() }
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
                Activate(_params) => todo!(),
                Authenticate(_params) => todo!(),
                Next(_params) => todo!(),
                GetAcl(_params) => todo!(),
                GenKey(_params) => todo!(),
                Revert(_params) => todo!(),
                RevertSp(_params) => todo!(),
                Random(params) => MethodResult(self.random(tper, invoking_id, params)).to_tokens(),
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
            Some(session_id.assign(Packet {
                payload: vec![SubPacket {
                    kind: SubPacketKind::Data,
                    length: std::marker::PhantomData,
                    payload: result_tokens.expect("failed to serialize method result"),
                }],
                ..Default::default()
            }))
        } else {
            None
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

        let Some(access_control) = ac_table.get(&AccessControlRef { invoking_id, method_id }) else {
            return Err(MethodStatus::NotAuthorized);
        };

        let ace_table = sp.ace();
        let mut permitted_columns = HashSet::new();
        for ace_ref in &access_control.acl {
            let ace = ace_table.get(ace_ref).expect("referenced ACE is missing from preconfig");
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

        if columns.all(|column| permitted_columns.contains(&column)) {
            Ok(())
        } else {
            Err(MethodStatus::NotAuthorized)
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
                    payload: call.to_tokens().expect("method tokenization failed"),
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
                    payload: call.to_tokens().expect("method tokenization failed"),
                }],
                ..Default::default()
            }))
        } else {
            None
        }
    }
}
