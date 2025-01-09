use std::collections::{HashMap, VecDeque as Queue};
use std::sync::{Arc, Mutex};

use crate::device::Error;
use crate::messaging::com_id::{
    ComIdRequestCode, ComIdState, HandleComIdRequest, HandleComIdResponse, StackResetResponsePayload, StackResetStatus,
    VerifyComIdValidResponsePayload,
};
use crate::messaging::packet::ComPacket;
use crate::messaging::uid::UID;
use crate::serialization::vec_with_len::VecWithLen;
use crate::serialization::{DeserializeBinary, SerializeBinary};

use super::controller::Controller;

pub struct Session {
    com_id: u16,
    com_id_ext: u16,
    controller: Arc<Mutex<Controller>>,
    com_queue: Queue<HandleComIdResponse>,
    packet_queue: Queue<ComPacket>,
    sp_sessions: HashMap<(u32, u32), Vec<UID>>,
}

impl Session {
    pub fn new(com_id: u16, com_id_ext: u16, controller: Arc<Mutex<Controller>>) -> Self {
        Self {
            com_id,
            com_id_ext,
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
        let response = self.process_com(request);
        self.com_queue.push_back(response);
        Ok(())
    }

    pub fn on_security_send_packet(&mut self, data: &[u8]) -> Result<(), Error> {
        let Ok(request) = ComPacket::from_bytes(data.into()) else {
            return Ok(());
        };
        let response = self.process_packet(request);
        self.packet_queue.push_back(response);
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
        let response = match self.packet_queue.front() {
            Some(response) => response,
            None => &no_packet_response(self.com_id, self.com_id_ext),
        };
        let mut bytes = response.to_bytes().expect("device shouldn't generate invalid responses");
        if bytes.len() <= len {
            self.com_queue.pop_front();
            bytes.resize(len, 0);
            Ok(bytes)
        } else {
            Err(Error::BufferTooShort)
        }
    }

    pub fn process_com(&mut self, request: HandleComIdRequest) -> HandleComIdResponse {
        match request.request_code {
            ComIdRequestCode::VerifyComIdValid => self.verify_com_id_valid(request.com_id, request.com_id_ext),
            ComIdRequestCode::StackReset => self.reset_stack(request.com_id, request.com_id_ext),
        }
    }

    pub fn process_packet(&mut self, _request: ComPacket) -> ComPacket {
        no_packet_response(self.com_id, self.com_id_ext)
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
