//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::ops::Deref;

use crate::call_with_tuple::CallSelfWithTuple;
use crate::device::Error;
use crate::fake_device::sp_session::SPSessionExecutor;
use crate::messaging::packet::{Packet, SubPacket, SubPacketKind};
use crate::messaging::token::Token;
use crate::messaging::value::Value;
use crate::rpc::args::{IntoMethodArgs, TryFromMethodArgs, UnwrapMethodArgs};
use crate::rpc::{
    MethodCall, MethodResult, MethodStatus, PackagedMethod, Properties, SessionIdentifier, CONTROL_SESSION_ID,
};
use crate::serialization::vec_without_len::VecWithoutLen;
use crate::serialization::{Deserialize, InputStream, OutputStream, Serialize};
use crate::spec::column_types::{MethodRef, SPRef};
use crate::spec::{invoking_id::*, method_id::*, sm_method_id::*};

use super::data::TPer;
use super::sp_session::SPSession;

enum DispatchError {
    Aborted,
    Closed,
}

pub struct SPSessionDispatcher {
    session_id: SessionIdentifier,
    properties: Properties,
    sp_session: Option<SPSession>,
    reverted_sps: Vec<SPRef>,
}

impl SPSessionDispatcher {
    pub fn new(session_id: SessionIdentifier, sp_session: SPSession, properties: Properties) -> Self {
        Self { session_id, properties, sp_session: Some(sp_session), reverted_sps: vec![] }
    }

    pub fn get_session_id(&self) -> SessionIdentifier {
        self.session_id
    }

    pub fn take_reverted_sps(&mut self) -> Vec<SPRef> {
        core::mem::replace(&mut self.reverted_sps, vec![])
    }

    pub fn dispatch(&mut self, tper: &mut TPer, packet: Packet) -> Vec<Packet> {
        assert_eq!(SessionIdentifier::from(&packet), self.session_id, "packet does not belong to this session");
        let Ok(packaged_methods) = split_packet(&packet) else {
            return vec![];
        };

        let mut sp_results = Vec::new();
        let mut sm_results = Vec::new();
        for packaged_method in packaged_methods {
            match self.dispatch_method(tper, packaged_method) {
                Ok(PackagedMethod::EndOfSession) => {
                    sp_results.push(PackagedMethod::EndOfSession);
                    self.sp_session = None;
                }
                Err(DispatchError::Aborted) => {
                    sm_results.push(PackagedMethod::Call(MethodCall {
                        invoking_id: SESSION_MANAGER,
                        method_id: CLOSE_SESSION,
                        args: (self.session_id.hsn, self.session_id.tsn).into_method_args(),
                        status: MethodStatus::Success,
                    }));
                    self.sp_session = None;
                }
                Err(DispatchError::Closed) => {
                    break;
                }
                Ok(result) => {
                    sp_results.push(result);
                }
            };
            if let Some(sp_session) = self.sp_session.as_mut() {
                let sp = sp_session.sp();
                let mut reverted_sps = sp_session.take_reverted_sps();
                if reverted_sps.contains(&sp) {
                    self.sp_session = None;
                }
                self.reverted_sps.append(&mut reverted_sps);
            }
        }

        let sp_packets = bundle_methods(self.session_id, sp_results.as_slice());
        let sm_packets = bundle_methods(CONTROL_SESSION_ID, sm_results.as_slice());
        sp_packets.into_iter().chain(sm_packets.into_iter()).collect()
    }

    fn dispatch_method(&mut self, tper: &mut TPer, method: PackagedMethod) -> Result<PackagedMethod, DispatchError> {
        let result = match method {
            PackagedMethod::Call(call) => self.dispatch_call(tper, call).map(|x| PackagedMethod::Result(x)),
            PackagedMethod::Result(_) => Err(DispatchError::Aborted),
            PackagedMethod::EndOfSession => Ok(PackagedMethod::EndOfSession),
        };
        result
    }

    pub fn dispatch_call(&mut self, tper: &mut TPer, call: MethodCall) -> Result<MethodResult, DispatchError> {
        let sp_session = self.sp_session.as_mut().ok_or(DispatchError::Closed)?;
        let Ok(method_id) = MethodRef::try_from(call.method_id) else {
            return Ok(MethodResult::new_fail(MethodStatus::InvalidParameter));
        };

        let args: Vec<_> = core::iter::once(Value::from(call.invoking_id)).chain(call.args.into_iter()).collect();
        let result = match method_id {
            AUTHENTICATE => call_method(sp_session, tper, SPSessionExecutor::authenticate, args),
            GET => call_method(sp_session, tper, SPSessionExecutor::get, args),
            SET => call_method(sp_session, tper, SPSessionExecutor::set, args),
            NEXT => call_method(sp_session, tper, SPSessionExecutor::next, args),
            GEN_KEY => call_method(sp_session, tper, SPSessionExecutor::gen_key, args),
            GET_ACL => call_method(sp_session, tper, SPSessionExecutor::get_acl, args),
            REVERT => call_method(sp_session, tper, SPSessionExecutor::revert, args),
            REVERT_SP => call_method(sp_session, tper, SPSessionExecutor::revert_sp, args),
            ACTIVATE => call_method(sp_session, tper, SPSessionExecutor::activate, args),
            _ => MethodResult::new_fail(MethodStatus::InvalidParameter),
        };
        Ok(result)
    }
}

fn call_method<'session, 'tper, Function, Output, Tuple>(
    session: &'session mut SPSession,
    tper: &'tper mut TPer,
    f: Function,
    args: Vec<Value>,
) -> MethodResult
where
    for<'exec> Function:
        CallSelfWithTuple<&'exec mut SPSessionExecutor<'session, 'tper>, Result<Output, MethodStatus>, Tuple>,
    Tuple: TryFromMethodArgs<Error = MethodStatus>,
    Output: IntoMethodArgs,
{
    let result = match args.unwrap_method_args() {
        Ok(args) => f.call_self_with_tuple(&mut session.on_tper(tper), args),
        Err(_) => Err(MethodStatus::InvalidParameter),
    };
    match result {
        Ok(values) => MethodResult { results: values.into_method_args(), status: MethodStatus::Success },
        Err(status) => MethodResult::new_fail(status),
    }
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
