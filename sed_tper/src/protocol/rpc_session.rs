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
    protocol::{
        management::Management,
        sequence_number::SequenceNumber,
        session::Session,
        shared::{PropertiesChanged, min_deadline},
    },
};

use super::{
    management::{Action as ManagementAction, StackAction},
    session::Action as SessionAction,
};

#[derive(Debug)]
pub struct RpcSession {
    timeout: Duration,
    packets_to_send: VecDeque<Vec<Packet>>,
    /// `StartSession` methods that are queued for IF-SEND, keyed by HSN.
    management: Management,
    /// Session method calls that are queued for IF-SEND, keyed by session ID.
    sessions: HashMap<SessionId, Session>,
}

impl RpcSession {
    pub fn new(timeout: Duration, capabilities: Properties) -> Self {
        Self {
            timeout,
            packets_to_send: VecDeque::new(),
            management: Management::new(timeout, capabilities),
            sessions: HashMap::new(),
        }
    }

    pub fn handle_method_call(&mut self, session_id: SessionId, call: Vec<u8>, sender: Sender<Result<Vec<u8>, Error>>) {
        if session_id == SessionId::MANAGEMENT {
            self.management.handle_method_call(call, sender);
        } else if let Some(session) = self.sessions.get_mut(&session_id) {
            session.handle_method_call(call, sender);
        } else {
            let _ = sender.send(Err(Error::Closed));
        }
    }

    pub fn handle_sync_properties(&mut self) {
        self.management.handle_sync_properties();
    }

    pub fn handle_session_aborted(&mut self, session_id: SessionId) {
        if let Some(mut session) = self.sessions.remove(&session_id) {
            session.handle_aborted();
        }
    }

    pub fn handle_reset(&mut self) {
        let capabilities = self.management.capabilities().clone();
        self.management.handle_reset();
        for (_, mut session) in self.sessions.drain() {
            session.handle_aborted();
        }
        *self = Self::new(self.timeout, capabilities);
    }

    #[cfg(feature = "test-utils")]
    pub fn handle_spawn_session(&mut self, session_id: SessionId, properties: Properties) {
        self.sessions.entry(session_id).or_insert(Session::new(session_id, self.timeout, properties));
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
                        StackAction::Spawn { session_id, properties } => {
                            self.sessions.insert(session_id, Session::new(session_id, self.timeout, properties));
                        }
                        StackAction::NotifyAbort { session_id } => {
                            self.sessions.remove(&session_id).map(|mut session| session.handle_aborted());
                        }
                        StackAction::Properties { .. } => {
                            // TODO: send out a properties changed event.
                            // Getting this working can wait.
                        }
                    }
                }
            } else if let Some(session) = self.sessions.get_mut(&session_id) {
                session.handle_tokens(sub_packet.payload);
            }
        }
    }

    pub fn poll_action(&mut self, time: Instant) -> RpcAction {
        let management_action = self.management.poll_action(time);
        let session_actions: Vec<_> = self
            .sessions
            .iter_mut()
            .map(|(session_id, session)| (*session_id, session.poll_action(time)))
            .collect();

        let (packets_to_send, deleted_sessions, deadline) = reduce_actions(management_action, session_actions);

        for session_id in deleted_sessions {
            let _ = self.sessions.remove(&session_id);
        }
        self.packets_to_send.extend(packets_to_send);

        if let Some(packets) = self.packets_to_send.pop_front() {
            RpcAction::Send(packets)
        } else if let Some(deadline) = deadline {
            RpcAction::Sleep { until: deadline }
        } else {
            RpcAction::None
        }
    }

    pub fn properties_changed(&self) -> async_broadcast::Receiver<PropertiesChanged> {
        self.management.properties_changed()
    }
}

fn reduce_actions(
    management_action: ManagementAction,
    session_actions: Vec<(SessionId, SessionAction)>,
) -> (Vec<Vec<Packet>>, Vec<SessionId>, Option<Instant>) {
    let mut packets_to_send = Vec::new();
    let mut deleted_sessions = Vec::new();

    let mut deadline = match management_action {
        ManagementAction::None => None,
        ManagementAction::Sleep { until } => Some(until),
        ManagementAction::Send(packets) => {
            packets_to_send.push(packets);
            None
        }
    };

    for (session_id, action) in session_actions {
        let session_deadline = match action {
            SessionAction::None => None,
            SessionAction::Sleep { until } => Some(until),
            SessionAction::Send(packets) => {
                packets_to_send.push(packets);
                None
            }
            SessionAction::Delete => {
                deleted_sessions.push(session_id);
                None
            }
        };
        deadline = min_deadline(deadline, session_deadline);
    }
    (packets_to_send, deleted_sessions, deadline)
}

#[derive(Debug, PartialEq, Eq)]
pub enum RpcAction {
    None,
    Sleep { until: Instant },
    Send(Vec<Packet>),
}

#[cfg(test)]
mod tests {
    use googletest::assert_that;
    use googletest::matchers::*;
    use oneshot::channel;
    use sed_spec::methods::MethodStatus;

    use crate::protocol::shared::eos;
    use crate::protocol::shared::packetize_one;
    use crate::protocol::shared::tests::*;

    use super::*;

    const SESSION_ID_1: SessionId = SessionId { hsn: 1, tsn: 2 };
    const SESSION_ID_2: SessionId = SessionId { hsn: 4, tsn: 5 };
    const TIMEOUT: Duration = Duration::from_secs(1);
    const CAPABILITIES: Properties = Properties::INITIAL;

    fn construct_with_sessions(sessions: &[SessionId]) -> RpcSession {
        let mut session = RpcSession::new(TIMEOUT, CAPABILITIES);
        for session_id in sessions {
            session.sessions.insert(*session_id, Session::new(*session_id, TIMEOUT, Properties::INITIAL));
        }
        session
    }

    #[test]
    fn start_session_successfully() {
        let mut session = RpcSession::new(TIMEOUT, CAPABILITIES);
        let time = Instant::now();
        let (sender, receiver) = channel();

        let start_call = start_session_call(SESSION_ID_1);
        let sync_call = sync_session_call(SESSION_ID_1, MethodStatus::Success);

        session.handle_method_call(SessionId::MANAGEMENT, start_call.clone(), sender);
        assert_that!(
            session.poll_action(time),
            pat!(RpcAction::Send(eq(&vec![packetize_one(
                SessionId::MANAGEMENT,
                SequenceNumber(1),
                start_call
            )])))
        );

        session.handle_iface_send_done(time, SessionId::MANAGEMENT, SequenceNumber(1), Ok(()));
        assert_that!(session.poll_action(time), pat!(&RpcAction::Sleep { until: eq(time + TIMEOUT) }));

        session.handle_packet(packetize_one(SessionId::MANAGEMENT, SequenceNumber(1), sync_call.clone()));
        assert_that!(session.poll_action(time), pat!(&RpcAction::None));

        assert_that!(receiver.try_recv(), ok(ok(eq(&sync_call))));
        assert!(session.sessions.contains_key(&SESSION_ID_1));
    }

    #[test]
    fn send_session_method_successfully() {
        let mut session = construct_with_sessions(&[SESSION_ID_1]);
        let time = Instant::now();
        let (sender, receiver) = channel();

        let call = method_call();
        let response = method_response();

        session.handle_method_call(SESSION_ID_1, call.clone(), sender);
        assert_that!(
            session.poll_action(time),
            pat!(RpcAction::Send(eq(&vec![packetize_one(SESSION_ID_1, SequenceNumber(1), call)])))
        );

        session.handle_iface_send_done(time, SESSION_ID_1, SequenceNumber(1), Ok(()));
        assert_that!(session.poll_action(time), pat!(&RpcAction::Sleep { until: eq(time + TIMEOUT) }));

        session.handle_packet(packetize_one(SESSION_ID_1, SequenceNumber(1), response.clone()));
        assert_that!(session.poll_action(time), pat!(&RpcAction::None));

        assert_that!(receiver.try_recv(), ok(ok(eq(&response))));
    }

    #[test]
    fn send_eos_successfully() {
        let mut session = construct_with_sessions(&[SESSION_ID_1]);
        let time = Instant::now();
        let (sender, receiver) = channel();

        let call = eos();
        let response = eos();

        session.handle_method_call(SESSION_ID_1, call.clone(), sender);
        assert_that!(
            session.poll_action(time),
            pat!(RpcAction::Send(eq(&vec![packetize_one(SESSION_ID_1, SequenceNumber(1), call)])))
        );

        session.handle_iface_send_done(time, SESSION_ID_1, SequenceNumber(1), Ok(()));
        assert_that!(session.poll_action(time), pat!(&RpcAction::Sleep { until: eq(time + TIMEOUT) }));

        session.handle_packet(packetize_one(SESSION_ID_1, SequenceNumber(1), response.clone()));
        assert_that!(session.poll_action(time), pat!(&RpcAction::None));

        assert_that!(receiver.try_recv(), ok(ok(eq(&response))));
        assert!(session.sessions.is_empty());
    }

    #[test]
    fn receive_close_session() {
        let mut session = construct_with_sessions(&[SESSION_ID_1]);
        let time = Instant::now();

        session.handle_packet(packetize_one(
            SessionId::MANAGEMENT,
            SequenceNumber(32),
            close_session_call(SESSION_ID_1),
        ));
        assert_that!(session.poll_action(time), pat!(&RpcAction::None));

        assert!(session.sessions.is_empty());
    }

    #[test]
    fn reduce_actions_deadline_management_first() {
        let time = Instant::now();
        let delay = Duration::from_secs(1);
        let management_action = ManagementAction::Sleep { until: time };
        let session_actions = vec![
            (SESSION_ID_1, SessionAction::Sleep { until: time + delay }),
            (SESSION_ID_2, SessionAction::Sleep { until: time + 2 * delay }),
        ];

        let (_, _, deadline) = reduce_actions(management_action, session_actions);
        assert_eq!(deadline, Some(time));
    }

    #[test]
    fn reduce_actions_deadline_session_first() {
        let time = Instant::now();
        let delay = Duration::from_secs(1);
        let management_action = ManagementAction::Sleep { until: time + 2 * delay };
        let session_actions = vec![
            (SESSION_ID_1, SessionAction::Sleep { until: time }),
            (SESSION_ID_2, SessionAction::Sleep { until: time + delay }),
        ];

        let (_, _, deadline) = reduce_actions(management_action, session_actions);
        assert_eq!(deadline, Some(time));
    }
}
