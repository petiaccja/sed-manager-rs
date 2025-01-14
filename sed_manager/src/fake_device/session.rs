use std::collections::{HashMap, VecDeque as Queue};
use std::ops::Deref;
use std::sync::{Arc, Mutex};

use crate::device::Error;
use crate::messaging::com_id::{
    ComIdRequestCode, ComIdState, HandleComIdRequest, HandleComIdResponse, StackResetResponsePayload, StackResetStatus,
    VerifyComIdValidResponsePayload,
};
use crate::messaging::packet::{ComPacket, Packet, SubPacket, SubPacketKind};
use crate::messaging::token::Token;
use crate::messaging::types::{List, MaxBytes32, NamedValue};
use crate::messaging::uid::UID;
use crate::rpc::{decode_args, encode_args, MethodCall, MethodStatus, Properties};
use crate::serialization::vec_with_len::VecWithLen;
use crate::serialization::vec_without_len::VecWithoutLen;
use crate::serialization::{Deserialize, DeserializeBinary, InputStream, OutputStream, Serialize, SerializeBinary};
use crate::specification::{invokers, methods};

use super::controller::Controller;

pub struct Session {
    com_id: u16,
    com_id_ext: u16,
    capabilities: Properties,
    properties: Properties,
    controller: Arc<Mutex<Controller>>,
    com_queue: Queue<HandleComIdResponse>,
    packet_queue: Queue<ComPacket>,
    sp_sessions: HashMap<(u32, u32), Vec<UID>>,
}

impl Session {
    pub fn new(com_id: u16, com_id_ext: u16, capabilities: Properties, controller: Arc<Mutex<Controller>>) -> Self {
        Self {
            com_id,
            com_id_ext,
            capabilities,
            properties: Properties::ASSUMED,
            controller,
            com_queue: Queue::new(),
            packet_queue: Queue::new(),
            sp_sessions: HashMap::new(),
        }
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

    pub fn on_security_recv_com(&mut self, len: usize) -> Result<Vec<u8>, Error> {
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

    pub fn on_security_recv_packet(&mut self, len: usize) -> Result<Vec<u8>, Error> {
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
            todo!()
        } else {
            Vec::new()
        }
    }

    fn process_control_session_packet(&mut self, request: Packet) -> Vec<Packet> {
        if let Ok(calls) = split_packet(&request) {
            let results: Vec<_> =
                calls.into_iter().filter_map(|call| self.process_control_session_call(call)).collect();
            let packets: Vec<_> = results
                .into_iter()
                .map(|result| -> Vec<Token> {
                    let mut stream = OutputStream::<Token>::new();
                    result.serialize(&mut stream).expect("responses should always be valid tokens");
                    stream.take()
                })
                .map(|tokens| -> Vec<u8> {
                    let mut stream = OutputStream::<u8>::new();
                    VecWithoutLen::from(tokens)
                        .serialize(&mut stream)
                        .expect("responses should always be valid tokens");
                    stream.take()
                })
                .map(|bytes| Packet {
                    host_session_number: request.host_session_number,
                    tper_session_number: request.tper_session_number,
                    payload: vec![SubPacket { kind: SubPacketKind::Data, payload: bytes.into() }].into(),
                    ..Default::default()
                })
                .collect();
            packets
        } else {
            Vec::new()
        }
    }

    fn process_control_session_call(&mut self, call: MethodCall) -> Option<MethodCall> {
        if call.invoking_id != invokers::SMUID {
            return None;
        }
        match call.method_id {
            methods::PROPERTIES => {
                if let Ok((host_properties,)) = decode_args!(call.args, Option<List<NamedValue<MaxBytes32, u32>>>) {
                    match self.properties(host_properties) {
                        Ok(results) => Some(MethodCall {
                            invoking_id: invokers::SMUID,
                            method_id: methods::PROPERTIES,
                            args: encode_args!(results.0, results.1),
                            status: MethodStatus::Success,
                        }),
                        Err(err) => Some(MethodCall {
                            invoking_id: invokers::SMUID,
                            method_id: methods::PROPERTIES,
                            args: vec![],
                            status: err,
                        }),
                    }
                } else {
                    None
                }
            }
            _ => None,
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

fn split_packet(packet: &Packet) -> Result<Vec<MethodCall>, Error> {
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
    let calls = match VecWithoutLen::<MethodCall>::deserialize(&mut token_stream) {
        Ok(calls) => calls.into_vec(),
        Err(_) => return Err(Error::InvalidArgument),
    };
    Ok(calls)
}
