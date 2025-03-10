use core::task::Poll;
use std::collections::BTreeMap;

use assemble_method::AssembleMethod;
pub use commit::commit;
use deserialize_sub_packet::deserialize_sub_packet;
use filter_close_session::filter_close_session;
use flatten_packet::flatten_packet;
use tokio::sync::oneshot;

use crate::messaging::packet::{ComPacket, Packet};
use crate::rpc::{Error, PackagedMethod, Properties, SessionIdentifier};

use flatten_com_packet::flatten_com_packet;

use super::session_identifier::CONTROL_SESSION_ID;
use super::shared::buffer::Buffer;
use super::shared::distribute::distribute;
use super::shared::pipe::{SinkPipe, SourcePipe};
use super::shared::timeout::Timeout;

mod assemble_method;
mod commit;
mod deserialize_sub_packet;
mod filter_close_session;
mod flatten_com_packet;
mod flatten_packet;

pub type Sender = oneshot::Sender<Result<PackagedMethod, Error>>;

pub struct ReceivePacket {
    sessions: BTreeMap<SessionIdentifier, Session>,
    packet: Buffer<Packet>,
}

struct Session {
    id: SessionIdentifier,
    sender: Buffer<Sender>,                        // Channel to send results back to TPer API.
    packet: Buffer<flatten_packet::Input>,         // Input packets.
    sub_packet: Buffer<flatten_packet::Output>,    // Input packets broken into sub-packets.
    token: Buffer<deserialize_sub_packet::Output>, // Input sub-packets deserialized into tokens.
    method: Buffer<assemble_method::Output>,       // Input tokens assembled into methods.
    filtered: Buffer<assemble_method::Output>,     // Input methods without CloseSession calls.
    closed_sessions: Buffer<SessionIdentifier>,    // Subject sessions of CloseSession calls.
    in_time: Buffer<assemble_method::Output>,      // Input methods after timeout is applied.
    assemble_method: AssembleMethod,
    timeout: Timeout,
    tracing_span: tracing::Span,
}

impl ReceivePacket {
    pub fn new() -> Self {
        Self { sessions: BTreeMap::new(), packet: Buffer::new() }
    }

    pub fn open_session(&mut self, id: SessionIdentifier, properties: Properties) {
        if !self.sessions.contains_key(&id) {
            let session = Session::new(id, properties);
            self.sessions.insert(id, session);
        }
    }

    pub fn close_session(&mut self, id: SessionIdentifier) {
        if let Some(session) = self.sessions.get_mut(&id) {
            session.close();
        }
    }

    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    pub fn update(
        &mut self,
        sender: &mut dyn SourcePipe<(SessionIdentifier, Sender)>,
        com_packet: &mut dyn SourcePipe<ComPacket>,
        done: &mut dyn SinkPipe<SessionIdentifier>,
    ) {
        // Always distribute senders first.
        distribute(
            sender,
            &mut Self::distribution_table_sender(&mut self.sessions),
            |(id, _)| *id,
            |(_, sender)| sender,
        );

        // Then distribute packets.
        flatten_com_packet(com_packet, &mut self.packet);
        distribute(
            &mut self.packet,
            &mut Self::distribution_table_packet(&mut self.sessions),
            |packet| SessionIdentifier { hsn: packet.host_session_number, tsn: packet.tper_session_number },
            |packet| packet,
        );
        for (_, session) in &mut self.sessions {
            session.update();
        }
        self.cleanup_sessions(done);
    }

    fn distribution_table_packet<'a>(
        sessions: &'a mut BTreeMap<SessionIdentifier, Session>,
    ) -> BTreeMap<SessionIdentifier, &'a mut dyn SinkPipe<flatten_com_packet::Output>> {
        sessions
            .iter_mut()
            .map(|(id, session)| (*id, session.input() as &mut dyn SinkPipe<flatten_com_packet::Output>))
            .collect()
    }

    fn distribution_table_sender<'a>(
        sessions: &'a mut BTreeMap<SessionIdentifier, Session>,
    ) -> BTreeMap<SessionIdentifier, &'a mut dyn SinkPipe<Sender>> {
        sessions
            .iter_mut()
            .map(|(id, session)| (*id, &mut session.sender as &mut dyn SinkPipe<Sender>))
            .collect()
    }

    fn cleanup_sessions(&mut self, sessions_done: &mut dyn SinkPipe<SessionIdentifier>) {
        let mut closed_sessions: Vec<_> = self
            .sessions
            .iter()
            .filter(|(_, session)| session.is_done() || session.is_aborted())
            .map(|(id, _)| *id)
            .collect();
        if let Some(control_session) = self.sessions.get_mut(&CONTROL_SESSION_ID) {
            while let Poll::Ready(Some(id)) = control_session.closed_sessions.pop() {
                closed_sessions.push(id);
            }
        }
        for id in &closed_sessions {
            self.sessions.remove(id);
            sessions_done.push(*id);
        }
        if !closed_sessions.is_empty() && self.sessions.is_empty() {
            sessions_done.close();
        }
    }
}

impl Session {
    pub fn new(id: SessionIdentifier, properties: Properties) -> Self {
        let tracing_span = tracing::span!(tracing::Level::DEBUG, "session", hsn = id.hsn, tsn = id.tsn);
        {
            let _guard = tracing_span.enter();
            tracing::event!(tracing::Level::DEBUG, "[recv] Started");
        };
        Self {
            id,
            sender: Buffer::new(),
            packet: Buffer::new(),
            sub_packet: Buffer::new(),
            token: Buffer::new(),
            method: Buffer::new(),
            filtered: Buffer::new(),
            closed_sessions: Buffer::new(),
            in_time: Buffer::new(),
            assemble_method: AssembleMethod::new(),
            timeout: Timeout::new(properties.trans_timeout),
            tracing_span,
        }
    }

    pub fn input(&mut self) -> &mut Buffer<flatten_packet::Input> {
        &mut self.packet
    }

    pub fn close(&mut self) {
        self.packet.close();
        self.sender.close();
    }

    pub fn update(&mut self) {
        let _guard = self.tracing_span.enter();
        if self.id != CONTROL_SESSION_ID {
            self.closed_sessions.close();
        }
        if self.sender.is_empty() {
            self.timeout.reset();
        }
        flatten_packet(&mut self.packet, &mut self.sub_packet);
        deserialize_sub_packet(&mut self.sub_packet, &mut self.token);
        self.assemble_method.update(&mut self.token, &mut self.method);
        filter_close_session(&mut self.method, &mut self.filtered, &mut self.closed_sessions);
        self.timeout.update(&mut self.filtered, &mut self.in_time, Some(|| Err(Error::TimedOut)));
        let num_comitted = commit(&mut self.sender, &mut self.in_time);
        if num_comitted > 0 {
            self.timeout.reset();
        }
        drop(_guard);
        self.restore_control_session();
    }

    pub fn is_done(&self) -> bool {
        commit::is_done(&self.sender, &self.in_time)
    }

    pub fn is_aborted(&self) -> bool {
        commit::is_aborted(&self.sender, &self.in_time)
    }

    fn restore_control_session(&mut self) {
        // The control session should not be terminated by errors.
        let is_control_session = self.id == CONTROL_SESSION_ID;
        let is_soft_closed = !self.sender.is_closed() && self.in_time.is_closed();
        if is_control_session && is_soft_closed {
            // Drop anything if queued and reopen buffers.
            self.token = Buffer::new();
            self.method = Buffer::new();
            self.filtered = Buffer::new();
            self.in_time = Buffer::new();
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _guard = self.tracing_span.enter();
        tracing::event!(tracing::Level::DEBUG, done = self.is_done(), aborted = self.is_aborted(), "[recv] Ended");
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use crate::messaging::packet::{SubPacket, SubPacketKind};
    use crate::messaging::token::{SerializeTokens, Tag};
    use crate::rpc::args::IntoMethodArgs;
    use crate::rpc::MethodCall;
    use crate::serialization::vec_without_len::VecWithoutLen;
    use crate::serialization::SerializeBinary;
    use crate::spec::invoking_id::SESSION_MANAGER;
    use crate::spec::sm_method_id;

    use super::*;

    const SHORT_TIMEOUT: Properties = Properties { trans_timeout: Duration::from_millis(1), ..Properties::ASSUMED };

    fn setup() -> (Buffer<(SessionIdentifier, Sender)>, Buffer<ComPacket>, ReceivePacket, Buffer<SessionIdentifier>) {
        (Buffer::new(), Buffer::new(), ReceivePacket::new(), Buffer::new())
    }

    fn make_com_packet(id: SessionIdentifier, valid: bool) -> ComPacket {
        let byte = if valid { Tag::EndOfSession as u8 } else { 0xFE };
        let sub_packet = SubPacket { kind: SubPacketKind::Data, payload: vec![byte].into() };
        let packet = Packet {
            host_session_number: id.hsn,
            tper_session_number: id.tsn,
            payload: vec![sub_packet].into(),
            ..Default::default()
        };
        ComPacket { com_id: 2048, com_id_ext: 0, outstanding_data: 0, min_transfer: 0, payload: vec![packet].into() }
    }

    fn make_close_session(to_close: SessionIdentifier) -> ComPacket {
        let method = PackagedMethod::Call(MethodCall::new_success(
            SESSION_MANAGER,
            sm_method_id::CLOSE_SESSION,
            (to_close.hsn, to_close.tsn).into_method_args(),
        ));
        let tokens = method.to_tokens().unwrap();
        let bytes = VecWithoutLen::from(tokens).to_bytes().unwrap();
        let sub_packet = SubPacket { kind: SubPacketKind::Data, payload: bytes.into() };
        let packet = Packet {
            host_session_number: 0,
            tper_session_number: 0,
            payload: vec![sub_packet].into(),
            ..Default::default()
        };
        ComPacket { com_id: 2048, com_id_ext: 0, outstanding_data: 0, min_transfer: 0, payload: vec![packet].into() }
    }

    fn make_channel() -> (Sender, oneshot::Receiver<Result<PackagedMethod, Error>>) {
        oneshot::channel()
    }

    #[test]
    fn initial_state() {
        let (mut sender, mut com_packet, mut node, mut done) = setup();
        node.update(&mut sender, &mut com_packet, &mut done);
        assert!(!done.is_closed());
    }

    #[test]
    fn invalid_session() {
        let (mut sender, mut com_packet, mut node, mut done) = setup();
        com_packet.push(make_com_packet(CONTROL_SESSION_ID, true));
        node.update(&mut sender, &mut com_packet, &mut done);
        assert!(!done.is_closed());
    }

    #[test]
    fn active_session() {
        let id = SessionIdentifier { hsn: 0, tsn: 0 };
        let (mut sender, mut com_packet, mut node, mut done) = setup();
        com_packet.push(make_com_packet(id, true));
        let (tx, mut rx) = make_channel();
        sender.push((id, tx));
        node.open_session(id, Properties::ASSUMED);
        node.update(&mut sender, &mut com_packet, &mut done);
        assert_eq!(rx.try_recv(), Ok(Ok(PackagedMethod::EndOfSession)));
        assert!(!done.is_closed());
    }

    #[test]
    fn active_session_abort_on_error() {
        let id = SessionIdentifier { hsn: 1, tsn: 1 };
        let (mut sender, mut com_packet, mut node, mut done) = setup();
        com_packet.push(make_com_packet(id, false));
        let (tx, mut rx) = make_channel();
        sender.push((id, tx));
        node.open_session(id, Properties::ASSUMED);
        node.update(&mut sender, &mut com_packet, &mut done);
        assert!(rx.try_recv().is_ok_and(|response| response.is_err()));
        assert_eq!(done.pop(), Poll::Ready(Some(id)));
        assert!(done.is_closed());
    }

    #[test]
    fn active_session_abort_by_tper() {
        let id = SessionIdentifier { hsn: 1, tsn: 1 };
        let (mut sender, mut com_packet, mut node, mut done) = setup();
        com_packet.push(make_close_session(id));
        let (tx, mut rx) = make_channel();
        sender.push((id, tx));
        node.open_session(CONTROL_SESSION_ID, Properties::ASSUMED);
        node.open_session(id, Properties::ASSUMED);
        node.update(&mut sender, &mut com_packet, &mut done);
        assert_eq!(rx.try_recv(), Err(oneshot::error::TryRecvError::Closed));
        assert_eq!(done.pop(), Poll::Ready(Some(id)));
        assert!(!done.is_closed());
    }

    #[test]
    fn active_session_restore_on_error() {
        let id = SessionIdentifier { hsn: 0, tsn: 0 };
        let (mut sender, mut com_packet, mut node, mut done) = setup();
        node.open_session(id, SHORT_TIMEOUT);

        com_packet.push(make_com_packet(id, false));
        let (tx, mut rx) = make_channel();
        sender.push((id, tx));
        node.update(&mut sender, &mut com_packet, &mut done);
        assert!(rx.try_recv().is_ok_and(|response| response.is_err()));
        assert!(!done.is_closed());

        com_packet.push(make_com_packet(id, true));
        let (tx, mut rx) = make_channel();
        sender.push((id, tx));
        node.update(&mut sender, &mut com_packet, &mut done);
        assert!(rx.try_recv().is_ok_and(|response| response.is_ok()));
        assert!(!done.is_closed());
    }

    #[test]
    fn closing_session() {
        let id = SessionIdentifier { hsn: 0, tsn: 0 };
        let (mut sender, mut com_packet, mut node, mut done) = setup();
        node.open_session(id, Properties::ASSUMED);
        node.update(&mut sender, &mut com_packet, &mut done);
        node.close_session(id);
        node.update(&mut sender, &mut com_packet, &mut done);
        assert!(done.is_closed());
    }
}
