//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

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
use tracing::Span;

use crate::{
    Error,
    protocol::{
        management::Management,
        sequence_number::SequenceNumber,
        session::Session,
        shared::{PacketBatch, PropertiesChanged, eos, min_deadline},
    },
};

use super::{management::ManagementAction, shared::PacketAction};

#[derive(Debug)]
pub struct RpcSession {
    timeout: Duration,
    packets_to_send: VecDeque<PacketBatch>,
    management: Management,
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

    pub fn handle_method_call(
        &mut self,
        session_id: SessionId,
        call: Vec<u8>,
        sender: Sender<Result<Vec<u8>, Error>>,
        span: Span,
    ) {
        if session_id == SessionId::MANAGEMENT {
            let _ = sender.send(Err(Error::UnexpectedManagementMethod));
        } else if let Some(session) = self.sessions.get_mut(&session_id) {
            session.handle_method_call(call, sender, span);
        } else {
            let _ = sender.send(Err(Error::Closed));
        }
    }

    pub fn handle_call_start_session(
        &mut self,
        call: sed_spec::methods::StartSession,
        sender: Sender<Result<(sed_spec::methods::SyncSession, Properties), Error>>,
        span: Span,
    ) {
        self.management.handle_call_start_session(call, sender, span);
    }

    pub fn handle_call_properties(&mut self, span: Span) {
        self.management.handle_call_properties(span);
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

    /// Process the packet received in the IF-RECV. The `source` is the span of
    /// the IF-RECV command.
    #[must_use]
    pub fn handle_packet(&mut self, packet: Packet, source: &Span) -> Option<RpcAction> {
        let session_id = SessionId::of(&packet);
        let mut rpc_action = None;
        for sub_packet in packet.payload.into_iter().filter(|s| s.kind == SubPacketKind::Data) {
            if session_id == SessionId::MANAGEMENT {
                for action in self.management.handle_tokens(sub_packet.payload, source) {
                    match action {
                        ManagementAction::Spawn { session_id, properties } => {
                            self.sessions.insert(session_id, Session::new(session_id, self.timeout, properties));
                        }
                        ManagementAction::SpawnOrphaned { session_id, properties } => {
                            let mut session = Session::new(session_id, self.timeout, properties);
                            let (tx, _rx) = oneshot::async_channel();
                            session.handle_method_call(eos(), tx, source.clone());
                            self.sessions.insert(session_id, session);
                        }
                        ManagementAction::NotifyAbort { session_id } => {
                            if let Some(mut session) = self.sessions.remove(&session_id) {
                                session.handle_aborted()
                            }
                        }
                        ManagementAction::StackReset { span } => {
                            rpc_action = Some(RpcAction::StackReset { span });
                        }
                    }
                }
            } else if let Some(session) = self.sessions.get_mut(&session_id) {
                session.handle_tokens(sub_packet.payload, source);
            }
        }
        rpc_action
    }

    pub fn poll_action(&mut self, time: Instant) -> PacketAction {
        let mut actions = Vec::new();
        actions.push(self.management.poll_action(time));
        actions.extend(self.sessions.values_mut().map(|session| session.poll_action(time)));
        self.sessions.retain(|_, session| !session.is_closed());

        let (packets_to_send, deadline) = reduce_actions(actions);
        self.packets_to_send.extend(packets_to_send);

        if let Some(packets) = self.packets_to_send.pop_front() {
            PacketAction::Send(packets)
        } else if let Some(deadline) = deadline {
            PacketAction::Sleep { until: deadline }
        } else {
            PacketAction::None
        }
    }

    /// The span of the method call that was sent the earliest among those
    /// awaiting a response.
    pub fn next_recv_span(&self) -> Option<(Instant, &Span)> {
        let management = self.management.next_recv_span();
        let sessions = self.sessions.values().map(|session| session.next_recv_span());
        core::iter::once(management).chain(sessions).flatten().min_by_key(|(sent_at, _)| *sent_at)
    }

    pub fn properties_changed(&self) -> async_broadcast::Receiver<PropertiesChanged> {
        self.management.properties_changed()
    }
}

fn reduce_actions(actions: Vec<PacketAction>) -> (Vec<PacketBatch>, Option<Instant>) {
    let mut packets_to_send = Vec::new();
    let mut deadline = None;

    for action in actions {
        let session_deadline = match action {
            PacketAction::None => None,
            PacketAction::Sleep { until } => Some(until),
            PacketAction::Send(packets) => {
                packets_to_send.push(packets);
                None
            }
        };
        deadline = min_deadline(deadline, session_deadline);
    }
    (packets_to_send, deadline)
}

#[must_use]
#[derive(Debug, PartialEq)]
pub enum RpcAction {
    StackReset { span: Span },
}

#[cfg(test)]
mod tests {
    use googletest::assert_that;
    use googletest::matchers::*;
    use oneshot::channel;
    use sed_packet::token_stream::ToTokens;
    use sed_spec::methods::MethodStatus;

    use crate::protocol::shared::eos;
    use crate::protocol::shared::packetize_one;
    use crate::protocol::shared::tests::*;

    use super::*;

    const SESSION_ID_1: SessionId = SessionId { hsn: 1, tsn: 2 };
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

        session.handle_call_start_session(start_call.parameters.clone(), sender, Span::current());
        assert_that!(
            session.poll_action(time),
            pat!(PacketAction::Send(elements_are![(
                eq(&packetize_one(SessionId::MANAGEMENT, SequenceNumber(1), start_call.to_tokens().unwrap())),
                anything()
            )]))
        );

        session.handle_iface_send_done(time, SessionId::MANAGEMENT, SequenceNumber(1), Ok(()));
        assert_that!(session.poll_action(time), pat!(&PacketAction::Sleep { until: eq(time + TIMEOUT) }));

        let action = session.handle_packet(
            packetize_one(SessionId::MANAGEMENT, SequenceNumber(1), sync_call.to_tokens().unwrap()),
            &Span::none(),
        );
        assert_that!(action, none());
        assert_that!(session.poll_action(time), pat!(&PacketAction::None));

        assert_that!(receiver.try_recv(), ok(ok(eq(&(sync_call.parameters, Properties::INITIAL)))));
        assert!(session.sessions.contains_key(&SESSION_ID_1));
    }

    #[test]
    fn send_session_method_successfully() {
        let mut session = construct_with_sessions(&[SESSION_ID_1]);
        let time = Instant::now();
        let (sender, receiver) = channel();

        let call = method_call();
        let response = method_response();

        session.handle_method_call(SESSION_ID_1, call.clone(), sender, Span::current());
        assert_that!(
            session.poll_action(time),
            pat!(PacketAction::Send(elements_are![(
                eq(&packetize_one(SESSION_ID_1, SequenceNumber(1), call)),
                anything()
            )]))
        );

        session.handle_iface_send_done(time, SESSION_ID_1, SequenceNumber(1), Ok(()));
        assert_that!(session.poll_action(time), pat!(&PacketAction::Sleep { until: eq(time + TIMEOUT) }));

        let action =
            session.handle_packet(packetize_one(SESSION_ID_1, SequenceNumber(1), response.clone()), &Span::none());
        assert_that!(action, none());
        assert_that!(session.poll_action(time), pat!(&PacketAction::None));

        assert_that!(receiver.try_recv(), ok(ok(eq(&response))));
    }

    #[test]
    fn send_eos_successfully() {
        let mut session = construct_with_sessions(&[SESSION_ID_1]);
        let time = Instant::now();
        let (sender, receiver) = channel();

        let call = eos();
        let response = eos();

        session.handle_method_call(SESSION_ID_1, call.clone(), sender, Span::current());
        assert_that!(
            session.poll_action(time),
            pat!(PacketAction::Send(elements_are![(
                eq(&packetize_one(SESSION_ID_1, SequenceNumber(1), call)),
                anything()
            )]))
        );

        session.handle_iface_send_done(time, SESSION_ID_1, SequenceNumber(1), Ok(()));
        assert_that!(session.poll_action(time), pat!(&PacketAction::Sleep { until: eq(time + TIMEOUT) }));

        let action =
            session.handle_packet(packetize_one(SESSION_ID_1, SequenceNumber(1), response.clone()), &Span::none());
        assert_that!(action, none());
        assert_that!(session.poll_action(time), pat!(&PacketAction::None));

        assert_that!(receiver.try_recv(), ok(ok(eq(&response))));
        assert!(session.sessions.is_empty());
    }

    #[test]
    fn close_orphaned_session() {
        let mut session = RpcSession::new(TIMEOUT, CAPABILITIES);
        let time = Instant::now();
        let (sender, receiver) = channel();

        let start_call = start_session_call(SESSION_ID_1);
        let sync_call = sync_session_call(SESSION_ID_1, MethodStatus::Success);

        // Start a new session.
        session.handle_call_start_session(start_call.parameters, sender, Span::current());
        assert_that!(session.poll_action(time), pat!(PacketAction::Send(len(eq(1)))));
        session.handle_iface_send_done(time, SessionId::MANAGEMENT, SequenceNumber(1), Ok(()));

        // Drop the receiver, orphaning the session.
        drop(receiver);

        // Finish the session startup via `SyncSession`.
        let rpc_action = session.handle_packet(
            packetize_one(SessionId::MANAGEMENT, SequenceNumber(1), sync_call.to_tokens().unwrap()),
            &Span::none(),
        );
        assert!(rpc_action.is_none());

        // The orphaned session should immediately send an eos.
        assert_that!(
            session.poll_action(time),
            pat!(PacketAction::Send(elements_are![(
                eq(&packetize_one(SESSION_ID_1, SequenceNumber(1), eos())),
                anything()
            )]))
        );
        // Feed the response to the EOS back to the orphaned session.
        session.handle_iface_send_done(time, SESSION_ID_1, SequenceNumber(1), Ok(()));
        let rpc_action = session.handle_packet(packetize_one(SESSION_ID_1, SequenceNumber(1), eos()), &Span::none());
        assert!(rpc_action.is_none());

        // There should be no more actions and the session should be deleted.
        assert_that!(session.poll_action(time), pat!(&PacketAction::None));
        assert!(session.sessions.is_empty());
    }

    #[test]
    fn reset_on_unexpected_session() {
        let mut session = RpcSession::new(TIMEOUT, CAPABILITIES);
        let time = Instant::now();

        let sync_call = sync_session_call(SESSION_ID_1, MethodStatus::Success);

        let rpc_action = session.handle_packet(
            packetize_one(SessionId::MANAGEMENT, SequenceNumber(1), sync_call.to_tokens().unwrap()),
            &Span::none(),
        );
        assert_that!(rpc_action, some(pat!(RpcAction::StackReset { .. })));
        assert_that!(session.poll_action(time), pat!(&PacketAction::None));
        assert!(session.sessions.is_empty());
    }

    #[test]
    fn not_allowed_calls_intercepted() {
        let mut session = construct_with_sessions(&[]);
        let time = Instant::now();
        let (sender, receiver) = channel();

        let call = start_session_call(SESSION_ID_1).to_tokens().unwrap();

        session.handle_method_call(SessionId::MANAGEMENT, call, sender, Span::current());
        assert_that!(session.poll_action(time), matches_pattern!(PacketAction::None));
        assert_that!(receiver.try_recv(), ok(err(pat!(&Error::UnexpectedManagementMethod))));
    }

    #[test]
    fn receive_close_session() {
        let mut session = construct_with_sessions(&[SESSION_ID_1]);
        let time = Instant::now();

        let action = session.handle_packet(
            packetize_one(SessionId::MANAGEMENT, SequenceNumber(32), close_session_call(SESSION_ID_1)),
            &Span::none(),
        );
        assert_that!(action, none());
        assert_that!(session.poll_action(time), pat!(&PacketAction::None));

        assert!(session.sessions.is_empty());
    }

    #[test]
    fn reduce_actions_earliest_deadline() {
        let time = Instant::now();
        let delay = Duration::from_secs(1);
        let actions = vec![
            PacketAction::Sleep { until: time + delay },
            PacketAction::Sleep { until: time },
            PacketAction::Sleep { until: time + 2 * delay },
        ];

        let (_, deadline) = reduce_actions(actions);
        assert_eq!(deadline, Some(time));
    }
}
