use std::collections::{HashMap, VecDeque as Queue};
use std::ops::Deref;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use crate::device::Error;
use crate::messaging::com_id::{
    ComIdRequestCode, ComIdState, HandleComIdRequest, HandleComIdResponse, StackResetResponsePayload, StackResetStatus,
    VerifyComIdValidResponsePayload,
};
use crate::messaging::packet::{ComPacket, Packet, SubPacket, SubPacketKind};
use crate::messaging::token::Token;
use crate::messaging::types::{List, MaxBytes32, NamedValue, RestrictedObjectReference, SPRef};
use crate::messaging::uid::UID;
use crate::messaging::value::Bytes;
use crate::rpc::args::{DecodeArgs, EncodeArgs};
use crate::rpc::{MethodCall, MethodResult, MethodStatus, PackagedMethod, Properties};
use crate::serialization::vec_with_len::VecWithLen;
use crate::serialization::vec_without_len::VecWithoutLen;
use crate::serialization::{Deserialize, DeserializeBinary, InputStream, OutputStream, Serialize, SerializeBinary};
use crate::specification::{invoker, method, table};

use super::data::SSC;
use super::sp_session::SPSession;

pub struct ComIDSession {
    com_id: u16,
    com_id_ext: u16,
    capabilities: Properties,
    properties: Properties,
    ssc: Arc<Mutex<SSC>>,
    com_queue: Queue<HandleComIdResponse>,
    packet_queue: Queue<ComPacket>,
    sp_sessions: HashMap<(u32, u32), SPSession>,
    next_tsn: AtomicU32,
}

impl ComIDSession {
    pub fn new(com_id: u16, com_id_ext: u16, capabilities: Properties, controller: Arc<Mutex<SSC>>) -> Self {
        Self {
            com_id,
            com_id_ext,
            capabilities,
            properties: Properties::ASSUMED,
            ssc: controller,
            com_queue: Queue::new(),
            packet_queue: Queue::new(),
            sp_sessions: HashMap::new(),
            next_tsn: 1.into(),
        }
    }

    pub fn active_sp_sessions(&self) -> Vec<(u32, u32, SPRef)> {
        self.sp_sessions.iter().map(|((hsn, tsn), sp_session)| (*hsn, *tsn, sp_session.sp())).collect()
    }

    pub fn on_security_send_com(&mut self, data: &[u8]) -> Result<(), Error> {
        let Ok(request) = HandleComIdRequest::from_bytes(data.into()) else {
            return Ok(());
        };
        let response = self.process_com_handle(request);
        self.com_queue.push_back(response);
        Ok(())
    }

    pub fn on_security_send_packet(&mut self, data: &[u8]) -> Result<(), Error> {
        let Ok(request) = ComPacket::from_bytes(data.into()) else {
            return Ok(());
        };
        let responses = self.process_com_packet(request);
        for com_packet in responses {
            self.packet_queue.push_back(com_packet);
        }
        Ok(())
    }

    pub fn on_security_recv_com(&mut self, len: usize) -> Result<Bytes, Error> {
        let response = match self.com_queue.front() {
            Some(response) => response,
            None => &no_com_id_response(self.com_id, self.com_id_ext),
        };
        let mut bytes = response.to_bytes().expect("device shouldn't generate invalid responses");
        if bytes.len() <= len {
            self.com_queue.pop_front();
        }
        bytes.resize(len, 0);
        Ok(bytes)
    }

    pub fn on_security_recv_packet(&mut self, len: usize) -> Result<Bytes, Error> {
        let response =
            if self.packet_queue.front().is_some_and(|com_packet| com_packet.get_transfer_len() as usize <= len) {
                self.packet_queue.pop_front().unwrap()
            } else {
                no_packet_response(self.com_id, self.com_id_ext)
            };
        let (outstanding_data, min_transfer) =
            self.packet_queue.front().map(|com_packet| (1, com_packet.get_transfer_len())).unwrap_or((0, 0));
        let response = ComPacket { outstanding_data, min_transfer, ..response };

        let mut bytes = response.to_bytes().expect("device shouldn't generate invalid responses");
        if bytes.len() <= len {
            bytes.resize(len, 0);
            Ok(bytes)
        } else {
            Err(Error::BufferTooShort)
        }
    }

    fn process_com_handle(&mut self, request: HandleComIdRequest) -> HandleComIdResponse {
        match request.request_code {
            ComIdRequestCode::VerifyComIdValid => self.verify_com_id_valid(request.com_id, request.com_id_ext),
            ComIdRequestCode::StackReset => self.reset_stack(request.com_id, request.com_id_ext),
        }
    }

    fn process_com_packet(&mut self, request: ComPacket) -> Vec<ComPacket> {
        let responses: Vec<_> =
            request.payload.into_vec().into_iter().map(|packet| self.process_packet(packet)).flatten().collect();
        let com_packets = responses
            .into_iter()
            .map(|packet| ComPacket {
                com_id: self.com_id,
                com_id_ext: self.com_id_ext,
                min_transfer: 0,
                outstanding_data: 0,
                payload: vec![packet].into(),
            })
            .collect();
        com_packets
    }

    fn process_packet(&mut self, request: Packet) -> Vec<Packet> {
        let (hsn, tsn) = (request.host_session_number, request.tper_session_number);
        if (hsn, tsn) == (0, 0) {
            self.process_control_session_packet(request)
        } else if let Some(_sp_session) = self.sp_sessions.get_mut(&(hsn, tsn)) {
            self.process_sp_session_packet(request)
        } else {
            Vec::new()
        }
    }

    fn process_control_session_packet(&mut self, request: Packet) -> Vec<Packet> {
        let (hsn, tsn) = (request.host_session_number, request.tper_session_number);
        if let Ok(calls) = split_packet(&request) {
            let results: Vec<_> =
                calls.into_iter().filter_map(|call| self.process_control_session_call(call)).collect();
            bundle_methods(hsn, tsn, results.as_slice())
        } else {
            Vec::new()
        }
    }

    fn process_sp_session_packet(&mut self, request: Packet) -> Vec<Packet> {
        let (hsn, tsn) = (request.host_session_number, request.tper_session_number);
        if let Ok(calls) = split_packet(&request) {
            let num_calls = calls.len();
            let results: Vec<_> =
                calls.into_iter().map_while(|call| self.process_sp_session_call(hsn, tsn, call)).collect();
            let mut packets = bundle_methods(hsn, tsn, results.as_slice());
            if results.len() != num_calls {
                if let Some(call) = self.abort_session(hsn, tsn) {
                    packets.append(&mut bundle_methods(0, 0, &[call]));
                }
            }
            packets
        } else {
            Vec::new()
        }
    }

    fn process_control_session_call(&mut self, call: PackagedMethod) -> Option<PackagedMethod> {
        let PackagedMethod::Call(call) = call else {
            return None;
        };
        if call.invoking_id != invoker::SMUID {
            return None;
        }
        match call.method_id {
            method::PROPERTIES => {
                if let Ok((_1,)) = call.args.decode_args() {
                    let result = self.properties(_1);
                    let call = format_response_call(invoker::SMUID, method::PROPERTIES, result);
                    Some(PackagedMethod::Call(call))
                } else {
                    None
                }
            }
            method::START_SESSION => {
                if let Ok((_1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12)) = call.args.decode_args() {
                    let result = self.start_session(_1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12);
                    let call = format_response_call(invoker::SMUID, method::SYNC_SESSION, result);
                    Some(PackagedMethod::Call(call))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn process_sp_session_call(&mut self, hsn: u32, tsn: u32, call: PackagedMethod) -> Option<PackagedMethod> {
        if let Some(sp_session) = self.sp_sessions.get_mut(&(hsn, tsn)) {
            match call {
                PackagedMethod::Call(call) => match call.method_id {
                    method::AUTHENTICATE => {
                        if let Ok((_1, _2)) = call.args.decode_args() {
                            let result = sp_session.authenticate(call.invoking_id, _1, _2);
                            let result = result.map(|x| (x,));
                            Some(PackagedMethod::Result(format_response_result(result)))
                        } else {
                            Some(format_response_failure(MethodStatus::InvalidParameter))
                        }
                    }
                    method::GET => {
                        if let Ok((_1,)) = call.args.decode_args() {
                            let result = sp_session.get(call.invoking_id, _1);
                            let result = result.map(|x| (x,));
                            Some(PackagedMethod::Result(format_response_result(result)))
                        } else {
                            Some(format_response_failure(MethodStatus::InvalidParameter))
                        }
                    }
                    method::SET => {
                        if let Ok((_1, _2)) = call.args.decode_args() {
                            let result = sp_session.set(call.invoking_id, _1, _2);
                            Some(PackagedMethod::Result(format_response_result(result)))
                        } else {
                            Some(format_response_failure(MethodStatus::InvalidParameter))
                        }
                    }
                    method::NEXT => {
                        if let Ok((_1, _2)) = call.args.decode_args() {
                            let result = sp_session.next(call.invoking_id, _1, _2);
                            let result = result.map(|x| (x,));
                            Some(PackagedMethod::Result(format_response_result(result)))
                        } else {
                            Some(format_response_failure(MethodStatus::InvalidParameter))
                        }
                    }
                    _ => Some(format_response_failure(MethodStatus::NotAuthorized)),
                },
                PackagedMethod::Result(_method_result) => self.abort_session(hsn, tsn),
                PackagedMethod::EndOfSession => self.close_session(hsn, tsn),
            }
        } else {
            None
        }
    }

    fn reset_stack(&mut self, com_id: u16, com_id_ext: u16) -> HandleComIdResponse {
        // In order to reset other sessions' stacks, the sessions would have to know about each other.
        // This is permitted by the spec, but I don't see a reason to implemented for only testing purposes.
        let payload = if com_id == self.com_id && self.com_id_ext == com_id_ext {
            self.com_queue.clear();
            self.packet_queue.clear();
            self.sp_sessions.clear();
            StackResetResponsePayload { stack_reset_status: StackResetStatus::Success }
        } else {
            StackResetResponsePayload { stack_reset_status: StackResetStatus::Failure }
        };
        HandleComIdResponse {
            com_id,
            com_id_ext,
            request_code: ComIdRequestCode::StackReset,
            payload: payload.to_bytes().unwrap().into(),
        }
    }

    fn verify_com_id_valid(&mut self, com_id: u16, com_id_ext: u16) -> HandleComIdResponse {
        // To report correct values for other com IDs, session would have to know about each other.
        // This is not worth implementing for a test device.
        let payload = if com_id == self.com_id && self.com_id_ext == com_id_ext {
            VerifyComIdValidResponsePayload { com_id_state: ComIdState::Associated }
        } else {
            VerifyComIdValidResponsePayload { com_id_state: ComIdState::Invalid }
        };
        HandleComIdResponse {
            com_id,
            com_id_ext,
            request_code: ComIdRequestCode::StackReset,
            payload: payload.to_bytes().unwrap().into(),
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
        _host_challenge: Option<Bytes>,
        _host_exch_auth: Option<RestrictedObjectReference<{ table::AUTHORITY.value() }>>,
        _host_exch_cert: Option<Bytes>,
        _host_sgn_auth: Option<RestrictedObjectReference<{ table::AUTHORITY.value() }>>,
        _host_sgn_cert: Option<Bytes>,
        _session_timeout: Option<u32>,
        _trans_timeout: Option<u32>,
        _initial_credit: Option<u32>,
        _signed_hash: Option<Bytes>,
    ) -> Result<
        (u32, u32, Option<Bytes>, Option<Bytes>, Option<Bytes>, Option<u32>, Option<u32>, Option<Bytes>),
        MethodStatus,
    > {
        let tsn = self.next_tsn.fetch_add(1, Ordering::Relaxed);
        let controller = self.ssc.lock().unwrap();
        if let Some(_sp) = controller.get_sp(sp_uid.into()) {
            let sp_session = SPSession::new(sp_uid, write, self.ssc.clone());
            self.sp_sessions.insert((hsn, tsn), sp_session);
            Ok((hsn, tsn, None, None, None, None, None, None))
        } else {
            Err(MethodStatus::InvalidParameter)
        }
    }

    fn close_session(&mut self, hsn: u32, tsn: u32) -> Option<PackagedMethod> {
        if let Some(_sp_session) = self.sp_sessions.remove(&(hsn, tsn)) {
            Some(PackagedMethod::EndOfSession)
        } else {
            None
        }
    }

    fn abort_session(&mut self, hsn: u32, tsn: u32) -> Option<PackagedMethod> {
        if let Some(_eos) = self.close_session(hsn, tsn) {
            Some(PackagedMethod::Call(MethodCall {
                invoking_id: invoker::SMUID,
                method_id: method::CLOSE_SESSION,
                args: (hsn, tsn).encode_args(),
                status: MethodStatus::Success,
            }))
        } else {
            None
        }
    }
}

fn no_com_id_response(com_id: u16, com_id_ext: u16) -> HandleComIdResponse {
    HandleComIdResponse {
        com_id,
        com_id_ext,
        request_code: crate::messaging::com_id::ComIdRequestCode::StackReset,
        payload: VecWithLen::new(),
    }
}

fn no_packet_response(com_id: u16, com_id_ext: u16) -> ComPacket {
    ComPacket { com_id, com_id_ext, min_transfer: 0, outstanding_data: 0, payload: VecWithLen::new() }
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

fn format_response_call<Args>(invoking_id: UID, method_id: UID, result: Result<Args, MethodStatus>) -> MethodCall
where
    Args: EncodeArgs,
{
    match result {
        Ok(args) => MethodCall { invoking_id, method_id, args: args.encode_args(), status: MethodStatus::Success },
        Err(status) => MethodCall { invoking_id, method_id, args: vec![], status },
    }
}

fn format_response_result<Args>(result: Result<Args, MethodStatus>) -> MethodResult
where
    Args: EncodeArgs,
{
    match result {
        Ok(args) => MethodResult { results: args.encode_args(), status: MethodStatus::Success },
        Err(status) => MethodResult { results: vec![], status },
    }
}

fn format_response_failure(status: MethodStatus) -> PackagedMethod {
    PackagedMethod::Result(MethodResult { results: Vec::new(), status })
}

fn bundle_methods(hsn: u32, tsn: u32, methods: &[PackagedMethod]) -> Vec<Packet> {
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
            host_session_number: hsn,
            tper_session_number: tsn,
            payload: vec![SubPacket { kind: SubPacketKind::Data, payload: bytes.into() }].into(),
            ..Default::default()
        })
        .collect()
}
