use std::collections::{HashMap, VecDeque};

use sed_device::Error;
use sed_packet::packet::ComPacket;
use sed_packet::session_id::SessionId;

use crate::com_id::{ComId, ComIdExt};
use crate::session::Session;
use crate::tper::TPer;

#[derive(Debug)]
pub struct PacketSession {
    com_id: ComId,
    com_id_ext: ComIdExt,
    is_associated: bool,
    sessions: HashMap<SessionId, Session>,
    response_queue: VecDeque<ComPacket>,
}

impl PacketSession {
    pub fn com_id_ext(&self) -> ComIdExt {
        self.com_id_ext
    }

    /// The state becomes "associated" when a new session is successfully started.
    /// This property is required to track ComID state.
    pub fn is_associated(&self) -> bool {
        self.is_associated
    }

    pub fn push(&mut self, tper: &mut TPer, com_packet: ComPacket) {
        for packet in com_packet.payload {
            let session_id = SessionId::of(&packet);
            if let Some(session) = self.sessions.get_mut(&session_id) {
                session.dispatch(tper, packet);
            }
        }
    }

    pub fn pop(&mut self, transfer_len: usize) -> Result<ComPacket, Error> {
        let front = self.response_queue.pop_front_if(|com_packet| com_packet.transfer_len() as usize <= transfer_len);
        match front {
            Some(front) => Ok(front),
            None => {
                let outstanding_data = self.response_queue.iter().map(|com_packet| com_packet.transfer_len()).sum();
                let min_transfer = self.response_queue.front().iter().map(|com_packet| com_packet.transfer_len()).sum();
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
