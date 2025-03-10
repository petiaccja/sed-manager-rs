use core::ops::Deref;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::call_with_tuple::CallSelfWithTuple;
use crate::device::Error;
use crate::messaging::packet::{Packet, SubPacket, SubPacketKind};
use crate::messaging::token::Token;
use crate::messaging::uid::UID;
use crate::messaging::value::{Bytes, Value};
use crate::rpc::args::{IntoMethodArgs, TryFromMethodArgs, UnwrapMethodArgs};
use crate::rpc::{MethodCall, MethodResult, MethodStatus, PackagedMethod, Properties, SessionIdentifier};
use crate::serialization::vec_without_len::VecWithoutLen;
use crate::serialization::{Deserialize, InputStream, OutputStream, Serialize};
use crate::spec::basic_types::{List, NamedValue};
use crate::spec::column_types::{AuthorityRef, MaxBytes32, MethodRef, SPRef};
use crate::spec::{invoking_id::*, method_id::*, sm_method_id::*};

use super::data::OpalV2Controller;
use super::security_provider_session::SecurityProviderSession;

pub struct PacketStack {
    controller: Arc<Mutex<OpalV2Controller>>,
    capabilities: Properties,
    properties: Properties,
    security_provider_sessions: HashMap<SessionIdentifier, SecurityProviderSession>,
}

impl PacketStack {
    pub fn new(capabilities: Properties, controller: Arc<Mutex<OpalV2Controller>>) -> Self {
        Self {
            controller,
            capabilities,
            properties: Properties::ASSUMED,
            security_provider_sessions: HashMap::new(),
        }
    }

    pub fn active_sessions(&self) -> Vec<(SessionIdentifier, SPRef)> {
        self.security_provider_sessions.iter().map(|(id, sp_session)| (*id, sp_session.this_sp())).collect()
    }

    pub fn dispatch_packet(&mut self, request: Packet) -> Vec<Packet> {
        let id = SessionIdentifier { hsn: request.host_session_number, tsn: request.tper_session_number };
        if id == (SessionIdentifier { hsn: 0, tsn: 0 }) {
            self.dispatch_packet_control_session(request)
        } else if let Some(_sp_session) = self.security_provider_sessions.get_mut(&id) {
            self.dispatch_packet_sp_session(request)
        } else {
            Vec::new()
        }
    }

    fn dispatch_packet_control_session(&mut self, request: Packet) -> Vec<Packet> {
        let id = SessionIdentifier { hsn: request.host_session_number, tsn: request.tper_session_number };
        if let Ok(calls) = split_packet(&request) {
            let results: Vec<_> =
                calls.into_iter().filter_map(|call| self.dispatch_request_control_session(call)).collect();
            bundle_methods(id, results.as_slice())
        } else {
            Vec::new()
        }
    }

    fn dispatch_packet_sp_session(&mut self, request: Packet) -> Vec<Packet> {
        let id = SessionIdentifier { hsn: request.host_session_number, tsn: request.tper_session_number };
        if let Ok(calls) = split_packet(&request) {
            let num_calls = calls.len();
            let results: Vec<_> =
                calls.into_iter().map_while(|call| self.dispatch_request_sp_session(id, call)).collect();
            let mut packets = bundle_methods(id, results.as_slice());
            if results.len() != num_calls {
                if let Some(call) = self.abort_session(id) {
                    packets.append(&mut bundle_methods(SessionIdentifier { hsn: 0, tsn: 0 }, &[call]));
                }
            }
            packets
        } else {
            Vec::new()
        }
    }

    fn dispatch_request_control_session(&mut self, request: PackagedMethod) -> Option<PackagedMethod> {
        let PackagedMethod::Call(call) = request else {
            return None;
        };
        if call.invoking_id != SESSION_MANAGER {
            return None;
        }
        match call.method_id {
            PROPERTIES => call_generic_session_manager(self, Self::properties, call.args, PROPERTIES),
            START_SESSION => call_generic_session_manager(self, Self::start_session, call.args, SYNC_SESSION),
            _ => None,
        }
    }

    fn dispatch_request_sp_session(
        &mut self,
        session_id: SessionIdentifier,
        request: PackagedMethod,
    ) -> Option<PackagedMethod> {
        let Some(sp_session) = self.security_provider_sessions.get_mut(&session_id) else {
            return None;
        };
        let mut call = match request {
            PackagedMethod::Call(call) => call,
            PackagedMethod::Result(_) => return self.abort_session(session_id),
            PackagedMethod::EndOfSession => return self.close_session(session_id),
        };

        let invalid_parameter = MethodResult { results: vec![], status: MethodStatus::InvalidParameter };
        let Ok(method_id) = MethodRef::try_from(call.method_id) else {
            return Some(PackagedMethod::Result(invalid_parameter));
        };

        let mut extended_args = vec![Value::from(call.invoking_id)];
        extended_args.append(&mut call.args);

        let result = match method_id {
            AUTHENTICATE => call_generic_sp_session(sp_session, SecurityProviderSession::authenticate, extended_args),
            GET => call_generic_sp_session(sp_session as &_, SecurityProviderSession::get, extended_args),
            SET => call_generic_sp_session(sp_session, SecurityProviderSession::set, extended_args),
            NEXT => call_generic_sp_session(sp_session as &_, SecurityProviderSession::next, extended_args),
            GEN_KEY => call_generic_sp_session(sp_session, SecurityProviderSession::gen_key, extended_args),
            GET_ACL => call_generic_sp_session(sp_session as &_, SecurityProviderSession::get_acl, extended_args),
            REVERT => call_generic_sp_session(sp_session, SecurityProviderSession::revert, extended_args),
            REVERT_SP => call_generic_sp_session(sp_session, SecurityProviderSession::revert_sp, extended_args),
            ACTIVATE => call_generic_sp_session(sp_session as &_, SecurityProviderSession::activate, extended_args),
            _ => Some(PackagedMethod::Result(invalid_parameter)),
        };
        let reverted = self.security_provider_sessions.get(&session_id).map(|s| s.reverted.clone()).unwrap_or(vec![]);
        self.abort_reverted_sp_sessions(reverted);
        result
    }

    pub fn abort_reverted_sp_sessions(&mut self, reverted: Vec<SPRef>) {
        let affected_sessions: Vec<_> = self
            .security_provider_sessions
            .iter()
            .filter(|(_, s)| reverted.contains(&s.this_sp()))
            .map(|(id, _)| *id)
            .collect();
        for id in affected_sessions {
            self.security_provider_sessions.remove(&id);
        }
    }

    pub fn properties(
        &mut self,
        host_properties: Option<List<NamedValue<MaxBytes32, u32>>>,
    ) -> Result<(List<NamedValue<MaxBytes32, u32>>, Option<List<NamedValue<MaxBytes32, u32>>>), MethodStatus> {
        let host_properties = host_properties.unwrap_or(List::new());
        let host_properties = Properties::from_list(host_properties.as_slice());
        let common_properties = Properties::common(&self.capabilities, &host_properties);
        let capabilities = self.capabilities.to_list();
        let common_properties = common_properties.to_list();
        Ok((capabilities, Some(common_properties)))
    }

    pub fn start_session(
        &mut self,
        hsn: u32,
        sp_uid: SPRef,
        write: bool,
        host_challenge: Option<Bytes>,
        host_exch_auth: Option<AuthorityRef>,
        host_exch_cert: Option<Bytes>,
        host_sgn_auth: Option<AuthorityRef>,
        host_sgn_cert: Option<Bytes>,
        session_timeout: Option<u32>,
        trans_timeout: Option<u32>,
        initial_credit: Option<u32>,
        signed_hash: Option<Bytes>,
    ) -> Result<
        (u32, u32, Option<Bytes>, Option<Bytes>, Option<Bytes>, Option<u32>, Option<u32>, Option<Bytes>),
        MethodStatus,
    > {
        let controller = self.controller.lock().unwrap();
        let result = controller.start_session(
            hsn,
            sp_uid,
            write,
            host_challenge,
            host_exch_auth,
            host_exch_cert,
            host_sgn_auth,
            host_sgn_cert,
            session_timeout,
            trans_timeout,
            initial_credit,
            signed_hash,
        );
        match result {
            Ok(sync_session) => {
                let id = SessionIdentifier { hsn: sync_session.0, tsn: sync_session.1 };
                let sp_session = SecurityProviderSession::new(sp_uid, write, self.controller.clone());
                self.security_provider_sessions.insert(id, sp_session);
                Ok(sync_session)
            }
            Err(err) => Err(err),
        }
    }

    fn close_session(&mut self, id: SessionIdentifier) -> Option<PackagedMethod> {
        if let Some(_sp_session) = self.security_provider_sessions.remove(&id) {
            Some(PackagedMethod::EndOfSession)
        } else {
            None
        }
    }

    fn abort_session(&mut self, id: SessionIdentifier) -> Option<PackagedMethod> {
        if let Some(_eos) = self.close_session(id) {
            Some(PackagedMethod::Call(MethodCall {
                invoking_id: SESSION_MANAGER,
                method_id: CLOSE_SESSION,
                args: (id.hsn, id.tsn).into_method_args(),
                status: MethodStatus::Success,
            }))
        } else {
            None
        }
    }
}

fn call_generic_session_manager<This, Function, Output, Tuple>(
    this: This,
    f: Function,
    args: Vec<Value>,
    response_method: UID,
) -> Option<PackagedMethod>
where
    Function: CallSelfWithTuple<This, Result<Output, MethodStatus>, Tuple>,
    Tuple: TryFromMethodArgs<Error = MethodStatus>,
    Output: IntoMethodArgs,
{
    let result = match args.unwrap_method_args() {
        Ok(args) => f.call_self_with_tuple(this, args),
        Err(_) => return None,
    };
    let invoking_id = SESSION_MANAGER;
    let response_call = match result {
        Ok(values) => MethodCall::new_success(invoking_id, response_method, values.into_method_args()),
        Err(status) => MethodCall { invoking_id, method_id: response_method, args: vec![], status },
    };
    Some(PackagedMethod::Call(response_call))
}

fn call_generic_sp_session<This, Function, Output, Tuple>(
    this: This,
    f: Function,
    args: Vec<Value>,
) -> Option<PackagedMethod>
where
    Function: CallSelfWithTuple<This, Result<Output, MethodStatus>, Tuple>,
    Tuple: TryFromMethodArgs<Error = MethodStatus>,
    Output: IntoMethodArgs,
{
    let result = match args.unwrap_method_args() {
        Ok(args) => f.call_self_with_tuple(this, args),
        Err(_) => Err(MethodStatus::InvalidParameter),
    };
    let response_result = match result {
        Ok(values) => MethodResult { results: values.into_method_args(), status: MethodStatus::Success },
        Err(status) => MethodResult { results: vec![], status },
    };
    Some(PackagedMethod::Result(response_result))
}

fn split_packet(packet: &Packet) -> Result<Vec<PackagedMethod>, Error> {
    let mut bytes = Vec::<u8>::new();
    for sub_packet in packet.payload.deref() {
        if sub_packet.kind == SubPacketKind::Data {
            bytes.append(&mut sub_packet.payload.clone());
        }
    }
    let mut byte_stream = InputStream::from(bytes);
    let tokens = match VecWithoutLen::<Token>::deserialize(&mut byte_stream) {
        Ok(tokens) => tokens.into_vec(),
        Err(_) => return Err(Error::InvalidArgument),
    };
    let mut token_stream = InputStream::from(tokens);
    let calls = match VecWithoutLen::<PackagedMethod>::deserialize(&mut token_stream) {
        Ok(calls) => calls.into_vec(),
        Err(_) => return Err(Error::InvalidArgument),
    };
    Ok(calls)
}

fn bundle_methods(id: SessionIdentifier, methods: &[PackagedMethod]) -> Vec<Packet> {
    methods
        .iter()
        .map(|result| -> Vec<Token> {
            let mut stream = OutputStream::<Token>::new();
            result.serialize(&mut stream).expect("responses should always be valid tokens");
            stream.take()
        })
        .map(|tokens| -> Vec<u8> {
            let mut stream = OutputStream::<u8>::new();
            VecWithoutLen::from(tokens).serialize(&mut stream).expect("responses should always be valid tokens");
            stream.take()
        })
        .map(|bytes| Packet {
            host_session_number: id.hsn,
            tper_session_number: id.tsn,
            payload: vec![SubPacket { kind: SubPacketKind::Data, payload: bytes.into() }].into(),
            ..Default::default()
        })
        .collect()
}
