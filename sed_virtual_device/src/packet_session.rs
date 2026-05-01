use std::collections::{HashMap, HashSet, VecDeque};

use sed_device::Error;
use sed_packet::packet::ComPacket;
use sed_packet::session_id::SessionId;
use sed_spec::methods::MethodStatus;
use sed_spec::objects::{AuthorityRef, SecurityProviderRef};
use sed_spec::preconfig::core::shared::authority::ANYBODY;

use crate::com_id::{ComId, ComIdExt};
use crate::management_session::ManagementSession;
use crate::session::Session;
use crate::tper::TPer;

#[derive(Debug)]
pub struct PacketSession {
    com_id: ComId,
    com_id_ext: ComIdExt,
    is_associated: bool,
    management_session: ManagementSession,
    sessions: HashMap<SessionId, Session>,
    response_queue: VecDeque<ComPacket>,
}

impl PacketSession {
    pub fn new(com_id: ComId, com_id_ext: ComIdExt) -> Self {
        Self {
            com_id,
            com_id_ext,
            is_associated: false,
            management_session: ManagementSession::new(),
            sessions: HashMap::new(),
            response_queue: VecDeque::new(),
        }
    }

    pub fn com_id_ext(&self) -> ComIdExt {
        self.com_id_ext
    }

    /// The state becomes "associated" when a new session is successfully started.
    /// This property is required to track ComID state.
    pub fn is_associated(&self) -> bool {
        self.is_associated
    }

    pub fn sessions(&self) -> HashSet<SessionId> {
        self.sessions.keys().cloned().collect()
    }

    /// Start a session without checking authentication.
    pub fn insert_session(
        &mut self,
        tper: &TPer,
        host_session_number: u32,
        sp: SecurityProviderRef,
        authority: Option<AuthorityRef>,
    ) -> Result<SessionId, MethodStatus> {
        let authority = authority.unwrap_or(ANYBODY);
        let tsn = self.management_session.next_tsn();
        let session_id = SessionId { hsn: host_session_number, tsn };
        self.sessions.insert(session_id, Session::new(tper, session_id, sp, authority)?);
        Ok(session_id)
    }

    pub fn push(&mut self, tper: &mut TPer, com_packet: ComPacket) {
        for packet in com_packet.payload {
            let session_id = SessionId::of(&packet);
            let packets = if session_id == SessionId::MANAGEMENT {
                self.management_session.dispatch(tper, &mut self.sessions, packet)
            } else if let Some(session) = self.sessions.get_mut(&session_id) {
                let packets = session.dispatch(tper, packet);
                if matches!(session, Session::Closed) {
                    self.sessions.remove(&session_id);
                }
                packets
            } else {
                vec![]
            };

            let com_packets =
                packets.into_iter().map(|packet| ComPacket { payload: vec![packet], ..Default::default() });
            self.response_queue.extend(com_packets);
        }
    }

    pub fn pop(&mut self, transfer_len: usize) -> Result<ComPacket, Error> {
        let front = self.response_queue.pop_front_if(|com_packet| com_packet.transfer_len() as usize <= transfer_len);
        let outstanding_data = self.response_queue.iter().map(|com_packet| com_packet.transfer_len()).sum();
        let min_transfer = self.response_queue.front().iter().map(|com_packet| com_packet.transfer_len()).sum();
        match front {
            Some(front) => Ok(ComPacket {
                com_id: self.com_id.0,
                com_id_ext: self.com_id_ext().0,
                outstanding_data,
                min_transfer,
                ..front
            }),
            None => {
                let response = ComPacket {
                    com_id: self.com_id.0,
                    com_id_ext: self.com_id_ext().0,
                    outstanding_data,
                    min_transfer,
                    length: std::marker::PhantomData,
                    payload: vec![],
                };
                if response.transfer_len() as usize <= transfer_len {
                    Ok(response)
                } else {
                    Err(Error::BufferTooShort)
                }
            }
        }
    }
}
