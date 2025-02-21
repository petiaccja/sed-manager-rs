use std::collections::VecDeque as Queue;
use std::sync::{Arc, Mutex};

use crate::device::Error;
use crate::messaging::com_id::{
    ComIdRequestCode, ComIdState, HandleComIdRequest, HandleComIdResponse, StackResetResponsePayload, StackResetStatus,
    VerifyComIdValidResponsePayload,
};
use crate::messaging::packet::ComPacket;
use crate::messaging::value::Bytes;
use crate::rpc::{Properties, SessionIdentifier};
use crate::serialization::vec_with_len::VecWithLen;
use crate::serialization::{DeserializeBinary, SerializeBinary};
use crate::spec::column_types::SPRef;

use super::data::OpalV2Controller;
use super::packet_stack::PacketStack;

pub struct ComIDSession {
    com_id: u16,
    com_id_ext: u16,
    capabilities: Properties,
    controller: Arc<Mutex<OpalV2Controller>>,
    com_queue: Queue<HandleComIdResponse>,
    packet_queue: Queue<ComPacket>,
    packet_stack: PacketStack,
}

impl ComIDSession {
    pub fn new(
        com_id: u16,
        com_id_ext: u16,
        capabilities: Properties,
        controller: Arc<Mutex<OpalV2Controller>>,
    ) -> Self {
        Self {
            com_id,
            com_id_ext,
            capabilities: capabilities.clone(),
            controller: controller.clone(),
            com_queue: Queue::new(),
            packet_queue: Queue::new(),
            packet_stack: PacketStack::new(capabilities, controller.clone()),
        }
    }

    pub fn active_sessions(&self) -> Vec<(SessionIdentifier, SPRef)> {
        self.packet_stack.active_sessions()
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
            ComIdRequestCode::NoResponseAvailable => {
                HandleComIdResponse { com_id: request.com_id, ..Default::default() }
            }
        }
    }

    fn process_com_packet(&mut self, request: ComPacket) -> Vec<ComPacket> {
        let responses: Vec<_> = request
            .payload
            .into_vec()
            .into_iter()
            .map(|packet| self.packet_stack.dispatch_packet(packet))
            .flatten()
            .collect();
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

    fn reset_stack(&mut self, com_id: u16, com_id_ext: u16) -> HandleComIdResponse {
        // In order to reset other sessions' stacks, the sessions would have to know about each other.
        // This is permitted by the spec, but I don't see a reason to implemented for only testing purposes.
        let payload = if com_id == self.com_id && self.com_id_ext == com_id_ext {
            self.com_queue.clear();
            self.packet_queue.clear();
            let _ = std::mem::replace(
                &mut self.packet_stack,
                PacketStack::new(self.capabilities.clone(), self.controller.clone()),
            );
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
