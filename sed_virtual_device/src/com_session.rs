use std::collections::HashMap;

use sed_packet::com_id::{
    ComIdRequest, ComIdRequestCode, ComIdResponse, ComIdResponsePayload, ComIdState, Date, StackResetStatus,
};

use crate::NUM_COM_IDS;
use crate::com_id::{ComId, ComIdExt};
use crate::device::BASE_COM_ID;
use crate::packet_session::PacketSession;

#[derive(Debug)]
pub struct ComSession {
    com_id: ComId,
    response_queue: Option<ComIdResponse>,
}

impl ComSession {
    pub fn new(com_id: ComId) -> Self {
        Self { com_id, response_queue: None }
    }

    pub fn push(&mut self, packet_sessions: &mut HashMap<ComId, PacketSession>, request: ComIdRequest) {
        match request.request_code {
            ComIdRequestCode::NoResponseAvailable => (),
            ComIdRequestCode::Verify => self.verify_com_id_valid(packet_sessions, &request),
            ComIdRequestCode::StackReset => self.stack_reset(packet_sessions, request),
        }
    }

    pub fn pop(&mut self) -> &ComIdResponse {
        self.response_queue.get_or_insert_with(|| ComIdResponse {
            com_id: self.com_id.0,
            com_id_ext: 0,
            payload: ComIdResponsePayload::NoResponseAvailable { available_data_length: 0 },
        })
    }

    fn verify_com_id_valid(&mut self, packet_sessions: &HashMap<ComId, PacketSession>, request: &ComIdRequest) {
        let com_id = ComId(request.com_id);
        let com_id_ext = ComIdExt(request.com_id_ext);
        let com_id_state = match packet_sessions.get(&com_id) {
            Some(session) => match session.com_id_ext() {
                value if value == com_id_ext => match session.is_associated() {
                    true => ComIdState::Associated,
                    false => ComIdState::Issued,
                },
                _ => ComIdState::Invalid,
            },
            None => ComIdState::Inactive,
        };

        let response = ComIdResponse {
            com_id: request.com_id,
            com_id_ext: request.com_id_ext,
            payload: ComIdResponsePayload::Verify {
                available_data_length: 0x22,
                com_id_state,
                time_of_allocation: Date::unsupported(),
                time_of_expiry: Date::unsupported(),
                time_since_reset: Date::unsupported(),
            },
        };

        // The currently queued response shall be discarded.
        let _ = self.response_queue.replace(response);
    }

    fn stack_reset(&mut self, packet_sessions: &mut HashMap<ComId, PacketSession>, request: ComIdRequest) {
        let com_id = ComId(request.com_id);
        let com_id_ext = ComIdExt(request.com_id_ext);

        let status = if let Some(session) = packet_sessions.get_mut(&com_id)
            && session.com_id_ext() == com_id_ext
        {
            let new_com_id_ext = increment_dynamic_com_id_ext(com_id, session.com_id_ext());
            *session = PacketSession::new(com_id, new_com_id_ext);
            StackResetStatus::Success
        } else {
            StackResetStatus::Failure
        };

        let response = ComIdResponse {
            com_id: request.com_id,
            com_id_ext: request.com_id,
            payload: ComIdResponsePayload::StackReset { available_data_length: 0x04, status },
        };

        // The currently queued response shall be discarded.
        let _ = self.response_queue.replace(response);
    }
}

fn increment_dynamic_com_id_ext(com_id: ComId, com_id_ext: ComIdExt) -> ComIdExt {
    if (BASE_COM_ID.0..BASE_COM_ID.0 + NUM_COM_IDS).contains(&com_id.0) {
        ComIdExt(0)
    } else {
        ComIdExt(com_id_ext.0.wrapping_add(1))
    }
}
