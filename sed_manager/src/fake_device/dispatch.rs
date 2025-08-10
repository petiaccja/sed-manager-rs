//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::ops::Deref as _;

use crate::call_with_tuple::CallSelfWithTuple;
use crate::device::Error;
use crate::fake_device::firmware::{Firmware, SPSession};
use crate::messaging::packet::{SubPacket, SubPacketKind};
use crate::messaging::token::Token;
use crate::messaging::uid::UID;
use crate::messaging::{packet::Packet, value::Value};
use crate::rpc::args::{IntoMethodArgs, TryFromMethodArgs, UnwrapMethodArgs as _};
use crate::rpc::{MethodCall, MethodResult, MethodStatus, PackagedMethod, SessionIdentifier, CONTROL_SESSION_ID};
use crate::serialization::vec_without_len::VecWithoutLen;
use crate::serialization::{Deserialize as _, InputStream, OutputStream, Serialize as _};
use crate::spec::column_types::MethodRef;
use crate::spec::{invoking_id, method_id, sm_method_id};

pub fn dispatch(firmware: &mut Firmware, packet: Packet) -> Vec<Packet> {
    let session_id = SessionIdentifier::from(&packet);
    if session_id == CONTROL_SESSION_ID {
        if let Ok(packaged_methods) = split_packet(&packet) {
            let mut sm_results = Vec::new();
            for packaged_method in packaged_methods {
                let result = match packaged_method {
                    PackagedMethod::Call(call) => dispatch_sm_method(firmware, call),
                    PackagedMethod::EndOfSession => continue, // Simply drop item.
                    PackagedMethod::Result(_) => continue,    // Simply drop item.
                };
                if let Some(call) = result {
                    sm_results.push(PackagedMethod::Call(call));
                }
            }
            bundle_methods(CONTROL_SESSION_ID, &sm_results)
        } else {
            // Unlike SP sessions, the control session simply drops invalid packets, and there is nothing else to do.
            vec![]
        }
    } else if let Some(mut session) = firmware.sp_session(session_id) {
        if let Ok(packaged_methods) = split_packet(&packet) {
            let mut abort = false;
            let mut sp_results = Vec::new();
            for packaged_method in packaged_methods {
                let result = match packaged_method {
                    PackagedMethod::Call(call) => PackagedMethod::Result(dispatch_sp_method(&mut session, call)),
                    PackagedMethod::EndOfSession => PackagedMethod::EndOfSession,
                    PackagedMethod::Result(_) => {
                        abort = true;
                        break;
                    }
                };
                sp_results.push(result);
            }

            let mut pruned_session_ids = firmware.take_pruned_session_ids();
            if abort {
                firmware.transient.remove_session(session_id);
                pruned_session_ids.push(session_id);
            }

            let sm_results: Vec<_> =
                pruned_session_ids.iter().map(|session_id| prepare_close_session(*session_id)).collect();
            let sp_packets = bundle_methods(session_id, &sp_results);
            let sm_packets = bundle_methods(CONTROL_SESSION_ID, &sm_results);
            sp_packets.into_iter().chain(sm_packets).collect()
        } else {
            firmware.transient.remove_session(session_id);
            let close_session = prepare_close_session(session_id);
            bundle_methods(CONTROL_SESSION_ID, &[close_session])
        }
    } else {
        vec![]
    }
}

fn dispatch_sm_method(firmware: &mut Firmware, call: MethodCall) -> Option<MethodCall> {
    use sm_method_id::*;

    if call.invoking_id != invoking_id::SESSION_MANAGER {
        return None;
    }

    let args = call.args;
    match call.method_id {
        PROPERTIES => call_sm_method(firmware, Firmware::properties, args, PROPERTIES),
        START_SESSION => call_sm_method(firmware, Firmware::start_session, args, SYNC_SESSION),
        _ => None,
    }
}

fn dispatch_sp_method(session: &mut SPSession, call: MethodCall) -> MethodResult {
    use method_id::*;

    let Ok(method_id) = MethodRef::try_from(call.method_id) else {
        return MethodResult::new_fail(MethodStatus::InvalidParameter);
    };

    let args: Vec<_> = core::iter::once(Value::from(call.invoking_id)).chain(call.args.into_iter()).collect();
    match method_id {
        AUTHENTICATE => call_sp_method(session, SPSession::authenticate, args),
        GET => call_sp_method(session, SPSession::get, args),
        SET => call_sp_method(session, SPSession::set, args),
        NEXT => call_sp_method(session, SPSession::next, args),
        GEN_KEY => call_sp_method(session, SPSession::gen_key, args),
        GET_ACL => call_sp_method(session, SPSession::get_acl, args),
        REVERT => call_sp_method(session, SPSession::revert, args),
        REVERT_SP => call_sp_method(session, SPSession::revert_sp, args),
        ACTIVATE => call_sp_method(session, SPSession::activate, args),
        _ => MethodResult::new_fail(MethodStatus::InvalidParameter),
    }
}

fn prepare_close_session(session_id: SessionIdentifier) -> PackagedMethod {
    let call = MethodCall::new_success(
        invoking_id::SESSION_MANAGER,
        sm_method_id::CLOSE_SESSION,
        (session_id.hsn, session_id.tsn).into_method_args(),
    );
    PackagedMethod::Call(call)
}

fn call_sm_method<'fw, Function, Output, Tuple>(
    firmware: &'fw mut Firmware,
    f: Function,
    args: Vec<Value>,
    response_method: UID,
) -> Option<MethodCall>
where
    Function: CallSelfWithTuple<&'fw mut Firmware, Result<Output, MethodStatus>, Tuple>,
    Tuple: TryFromMethodArgs<Error = MethodStatus>,
    Output: IntoMethodArgs,
{
    let result = match args.unwrap_method_args() {
        Ok(args) => f.call_self_with_tuple(firmware, args),
        Err(_) => return None,
    };
    let invoking_id = invoking_id::SESSION_MANAGER;
    let response_call = match result {
        Ok(values) => MethodCall::new_success(invoking_id, response_method, values.into_method_args()),
        Err(status) => MethodCall { invoking_id, method_id: response_method, args: vec![], status },
    };
    Some(response_call)
}

fn call_sp_method<'fw, 'session, Function, Output, Tuple>(
    session: &'session mut SPSession<'fw>,
    f: Function,
    args: Vec<Value>,
) -> MethodResult
where
    Function: CallSelfWithTuple<&'session mut SPSession<'fw>, Result<Output, MethodStatus>, Tuple>,
    Tuple: TryFromMethodArgs<Error = MethodStatus>,
    Output: IntoMethodArgs,
    'fw: 'session,
{
    let result = match args.unwrap_method_args() {
        Ok(args) => f.call_self_with_tuple(session, args),
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
