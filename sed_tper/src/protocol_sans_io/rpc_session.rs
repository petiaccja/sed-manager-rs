use std::{
    collections::{HashMap, VecDeque},
    time::{Duration, Instant},
};

use oneshot::Sender;
use sed_packet::{
    packet::{Packet, SubPacketKind},
    session_id::SessionId,
};
use sed_spec::methods::Properties;

use crate::{
    Error,
    protocol_sans_io::{
        management::Management, sequence_number::SequenceNumber, session::Session, utility::min_deadline,
    },
};

use super::{management::ManagementAction, session::SessionAction};

pub struct RpcSession {
    timeout: Duration,
    /// Queued packets (for now EOSs from aborted sessions).
    packets: VecDeque<Packet>,
    /// Method calls that haven't started through the pipeline yet.
    method_calls: VecDeque<MethodCallRecord>,
    /// `StartSession` methods that are queued for IF-SEND, keyed by HSN.
    management: Management,
    /// Session method calls that are queued for IF-SEND, keyed by session ID.
    sessions: HashMap<SessionId, Session>,
}

impl RpcSession {
    pub fn new(timeout: Duration, capabilities: Properties) -> Self {
        Self {
            timeout,
            packets: VecDeque::new(),
            method_calls: VecDeque::new(),
            management: Management::new(timeout, capabilities),
            sessions: HashMap::new(),
        }
    }

    pub fn handle_method_call(&mut self, session_id: SessionId, call: Vec<u8>, sender: Sender<Result<Vec<u8>, Error>>) {
        self.method_calls.push_back(MethodCallRecord { session_id, call, sender });
    }

    pub fn handle_iface_send_done(
        &mut self,
        time: Instant,
        session_id: SessionId,
        sn: SequenceNumber,
        result: Result<(), Error>,
    ) {
        if session_id == SessionId::MANAGEMENT {
            self.management.handle_iface_send_done(time, sn, result);
        } else if let Some(session) = self.sessions.get_mut(&session_id) {
            session.handle_iface_send_done(time, sn, result);
        }
    }

    pub fn handle_packet(&mut self, packet: Packet) {
        let session_id = SessionId::of(&packet);
        for sub_packet in packet.payload.into_iter().filter(|s| s.kind == SubPacketKind::Data) {
            if session_id == SessionId::MANAGEMENT {
                for action in self.management.handle_tokens(sub_packet.payload) {
                    match action {
                        ManagementAction::Spawn { session_id, properties } => {
                            self.sessions.insert(session_id, Session::new(session_id, self.timeout, properties));
                        }
                        ManagementAction::NotifyAbort { session_id } => {
                            self.sessions.remove(&session_id).map(|mut session| session.notify_abort());
                        }
                        ManagementAction::Properties { .. } => {
                            // TODO: send out a properties changed event.
                            // Getting this working can wait.
                        }
                    }
                }
            } else if let Some(session) = self.sessions.get_mut(&session_id) {
                match session.handle_tokens(sub_packet.payload) {
                    SessionAction::None => (),
                    SessionAction::Delete(packet) => {
                        self.packets.extend(packet);
                        drop(self.sessions.remove(&session_id));
                    }
                }
            }
        }
    }

    pub fn poll_packets(&mut self) -> Vec<Packet> {
        // One packet at a time for now.
        if let Some(packet) = self.packets.pop_front() {
            return vec![packet];
        };
        if let Some(MethodCallRecord { session_id, call, sender }) = self.method_calls.pop_front() {
            if session_id == SessionId::MANAGEMENT {
                self.management.handle_method_call(call, sender).into_iter().collect()
            } else if let Some(session) = self.sessions.get_mut(&session_id) {
                session.handle_method_call(call, sender).into_iter().collect()
            } else {
                let _ = sender.send(Err(Error::Closed));
                Vec::new()
            }
        } else {
            Vec::new()
        }
    }

    pub fn poll_timeout(&self) -> Option<Instant> {
        let mut deadline = self.management.poll_timeout();
        for (_, session) in &self.sessions {
            deadline = min_deadline(deadline, session.poll_timeout());
        }
        deadline
    }

    pub fn notify_time(&mut self, time: Instant) {
        self.management.notify_time(time);
        let mut actions = Vec::new();
        for (session_id, session) in &mut self.sessions {
            let action = session.notify_time(time);
            if action != SessionAction::None {
                actions.push((*session_id, action));
            }
        }
        for (session_id, action) in actions {
            match action {
                SessionAction::None => (),
                SessionAction::Delete(packet) => {
                    self.packets.extend(packet);
                    self.sessions.remove(&session_id);
                }
            }
        }
    }
}

struct MethodCallRecord {
    session_id: SessionId,
    call: Vec<u8>,
    sender: Sender<Result<Vec<u8>, Error>>,
}
