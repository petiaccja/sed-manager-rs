use std::collections::HashMap;

use sed_packet::com_id::{
    ComIdRequestCode, ComIdState, Date, HandleComIdRequest, HandleComIdResponse, HandleComIdResponseParams,
    StackResetStatus,
};

use crate::com_id::{ComId, ComIdExt};
use crate::packet_session::PacketSession;

#[derive(Debug)]
pub struct ComSession {
    com_id: ComId,
    response_queue: Option<HandleComIdResponse>,
}

impl ComSession {
    pub fn push(
        &mut self,
        packet_sessions: &HashMap<ComId, PacketSession>,
        request: HandleComIdRequest,
    ) -> Option<StackResetCommand> {
        match request.request_code {
            ComIdRequestCode::NoResponseAvailable => None,
            ComIdRequestCode::VerifyComIdValid => self.verify_com_id_valid(packet_sessions, &request),
            ComIdRequestCode::StackReset => self.stack_reset(packet_sessions, request),
        }
    }

    pub fn pop(&mut self) -> &HandleComIdResponse {
        self.response_queue.get_or_insert_with(|| HandleComIdResponse {
            com_id: self.com_id.0,
            com_id_ext: 0,
            params: HandleComIdResponseParams::NoResponseAvailable { available_data_length: 0 },
        })
    }

    fn verify_com_id_valid(
        &mut self,
        packet_sessions: &HashMap<ComId, PacketSession>,
        request: &HandleComIdRequest,
    ) -> Option<StackResetCommand> {
        let com_id = ComId(request.com_id);
        let com_id_ext = ComIdExt(request.com_id_ext);
        let session = packet_sessions.get(&com_id);
        let state = session.map(|session| (session.com_id_ext(), session.is_associated()));
        let com_id_state = match state {
            Some(params) if params == (com_id_ext, true) => ComIdState::Associated,
            Some(params) if params == (com_id_ext, false) => ComIdState::Issued,
            Some(_) => ComIdState::Invalid,
            None => ComIdState::Inactive,
        };

        let response = HandleComIdResponse {
            com_id: request.com_id,
            com_id_ext: request.com_id_ext,
            params: HandleComIdResponseParams::VerifyComIdValid {
                available_data_length: 0x22,
                com_id_state,
                time_of_allocation: Date::unsupported(),
                time_of_expiry: Date::unsupported(),
                time_since_reset: Date::unsupported(),
            },
        };

        // The currently queued response shall be discarded.
        let _ = self.response_queue.replace(response);
        None
    }

    fn stack_reset(
        &mut self,
        packet_sessions: &HashMap<ComId, PacketSession>,
        request: HandleComIdRequest,
    ) -> Option<StackResetCommand> {
        let com_id = ComId(request.com_id);
        let com_id_ext = ComIdExt(request.com_id_ext);
        let is_active = packet_sessions.get(&com_id).map(|session| session.com_id_ext() == com_id_ext).unwrap_or(false);
        let (command, status) = if is_active {
            (Some(StackResetCommand(com_id)), StackResetStatus::Success)
        } else {
            (None, StackResetStatus::Failure)
        };

        let response = HandleComIdResponse {
            com_id: request.com_id,
            com_id_ext: request.com_id,
            params: HandleComIdResponseParams::StackReset { available_data_length: 0x04, status },
        };

        // The currently queued response shall be discarded.
        let _ = self.response_queue.replace(response);
        command
    }
}

pub struct StackResetCommand(pub ComId);
