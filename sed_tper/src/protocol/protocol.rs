use core::mem::drop;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::num::NonZero;
use std::sync::Arc;
use std::time::{Duration, Instant};

use sed_async::{CancelToken, sleep_until, spawn, timeout_at};
use sed_device::Device;
use sed_packet::com_id::ComIdRequest;
use sed_packet::packet::{COM_PACKET_HEADER_LEN, PACKET_HEADER_LEN, SUB_PACKET_HEADER_LEN};
use sed_packet::session_id::SessionId;
use sed_spec::methods::{Limit, Properties};
use tracing::field::Empty;
use tracing::{Instrument, Span, debug_span, warn};

use crate::protocol::device_session::DeviceSession;
use crate::protocol::message::{
    ComResponse, ConnectionChanged, Message, MethodResponse, ReportAborted, SendComRequest, SendMethod,
};
use crate::protocol::{com_session::ComSession, management_session::ManagementSession, session::Session};

/// After the timeout, message sent between the host and device are considered
/// lost.
///
/// When ACK/NAK is not used, the protocol stack will simply use this value to
/// report a timeout in case the message still hasn't been received. This value
/// though is not communicated to the remote. I'm not really sure what's agreed
/// as a timeout in case of no ACK/NAK.
///
/// When ACK/NAK is used, messages need to be ACK-ed within the timeout. The
/// value of the timeout can be communicated in the `StartSession` method.
const DEF_TRANS_TIMEOUT: Duration = Duration::from_secs(if cfg!(feature = "test-utils") { 1 } else { 15 });

const MAX_GROSS_COM_PACKET_SIZE: usize = 1048576;

/// The capabilities supported by the protocol stack implementation.
///
/// Due to the complexities and ambiguities in the specification, asynchronous
/// communication, buffer management, ACK/NAK, and sequence numbers aren't
/// currently implemented. Additionally, most devices don't seem to support
/// these capabilities anyway.
///
/// The maximum packet sizes are generous as the PCs running this software have
/// plenty of RAM. A sensible limit is still necessary to prevent OOM in case
/// the device sends insane amounts of data due to a bug.
pub const CAPABILITIES: Properties = Properties {
    max_methods: Limit::Unlimited,
    max_subpackets: Limit::Unlimited,
    max_gross_packet_size: Limit::Limited(NonZero::new(MAX_GROSS_COM_PACKET_SIZE - COM_PACKET_HEADER_LEN).unwrap()),
    max_packets: Limit::Unlimited,
    max_gross_compacket_size: Limit::Limited(NonZero::new(MAX_GROSS_COM_PACKET_SIZE).unwrap()),
    max_gross_compacket_response_size: Limit::Limited(NonZero::new(MAX_GROSS_COM_PACKET_SIZE).unwrap()),
    max_sessions: None,
    max_read_sessions: None,
    max_ind_token_size: Limit::Limited(
        NonZero::new(MAX_GROSS_COM_PACKET_SIZE - COM_PACKET_HEADER_LEN - PACKET_HEADER_LEN - SUB_PACKET_HEADER_LEN)
            .unwrap(),
    ),
    max_agg_token_size: Limit::Limited(
        NonZero::new(MAX_GROSS_COM_PACKET_SIZE - COM_PACKET_HEADER_LEN - PACKET_HEADER_LEN - SUB_PACKET_HEADER_LEN)
            .unwrap(),
    ),
    max_authentications: None,
    max_transaction_limit: None,
    def_session_timeout: None,
    max_session_timeout: None,
    min_session_timeout: None,
    def_trans_timeout: None,
    max_trans_timeout: None,
    min_trans_timeout: None,
    max_com_id_time: None,
    continued_tokens: false,
    seq_numbers: false,
    ack_nak: false,
    asynchronous: false,
};

/// The full protocol to communicate with the TPer via packets and ComID requests.
#[derive(Debug)]
pub struct Protocol {
    device_session: DeviceSession,
    com_session: ComSession,
    management_session: ManagementSession,
    sessions: HashMap<SessionId, Session>,
    message_receiver: async_channel::Receiver<DispatchMessage>,
    connection_changed: async_broadcast::Sender<ConnectionChanged>,
}

impl Protocol {
    /// Create a new protocol stack for the `device` on the given ComID and
    /// ComID extension.
    ///
    /// This initializes the protocol stack, but no messages will be delivered
    /// until you call [`run`](Self::run).
    pub fn new(com_id: u16, com_id_ext: u16, device: Arc<dyn Device>) -> (Self, Controller) {
        let (message_tx, message_rx) = async_channel::unbounded();
        let (mut connection_tx, connection_rx) = async_broadcast::broadcast(1);
        connection_tx.set_overflow(true); // Using channel as "watch", only latest data should be cached.
        let controller = Controller { context: Context::new(message_tx), connection_changed: connection_rx };
        (
            Self {
                device_session: DeviceSession::new(com_id, com_id_ext, device, DEF_TRANS_TIMEOUT),
                com_session: ComSession::new(DEF_TRANS_TIMEOUT),
                management_session: ManagementSession::new(CAPABILITIES, DEF_TRANS_TIMEOUT),
                sessions: HashMap::new(),
                message_receiver: message_rx,
                connection_changed: connection_tx,
            },
            controller,
        )
    }

    /// Send and receive messages until the protocol stack is shut down.
    ///
    /// You typically want to spawn this as a task on an async runtime. While
    /// executing, the protocol stack will accept commands through
    /// [`Controller`]s and exchange the message with the device while
    /// respecting the communication protocols.
    ///
    /// To shut down the protocol stack, drop all [`Controller`]s. Once they are
    /// dropped, the protocol stack will still handle pending messages and
    /// timeouts to ensure a graceful shutdown. This will leave the protocol
    /// stack on the device's side ready for a subsequent session, but might
    /// take a little time.
    pub async fn run(mut self) {
        while let Ok(dispatch_message) = self.message_receiver.recv().await {
            self.dispatch(dispatch_message);
        }
    }

    fn dispatch(&mut self, DispatchMessage { address, context, message }: DispatchMessage) {
        let _span = context.root_span.clone().entered();
        match address {
            Address::Control => self.dispatch_control(context, message),
            Address::DeviceSession => self.dispatch_device_session(context, message),
            Address::ComSession => self.dispatch_com_session(context, message),
            Address::ManagementSession => self.dispatch_management_session(context, message),
            Address::Session(session_id) => self.dispatch_session(session_id, context, message),
        }
    }

    fn dispatch_control(&mut self, _context: Context, message: Message) {
        match message {
            Message::Spawn(message) => {
                let span = debug_span!(
                    "spawn_session",
                    session_id = debug(&message.id),
                    properties = debug(&message.properties),
                    err = Empty
                )
                .entered();
                match self.sessions.entry(message.id) {
                    Entry::Occupied(_) => drop(span.record("err", "session already exists")),
                    Entry::Vacant(entry) => {
                        let _ = entry.insert(Session::new(message.id, message.properties, DEF_TRANS_TIMEOUT));
                    }
                }
            }
            Message::Delete(message) => {
                let span = debug_span!("delete_session", session_id = debug(&message.0), err = Empty).entered();
                if self.sessions.remove(&message.0).is_none() {
                    span.record("err", "session not found");
                }
            }
            Message::ConnectionChanged(message) => {
                assert!(self.connection_changed.try_broadcast(message).is_ok(), "overflow mode disabled");
            }
            _ => warn!(message = debug(message), "message dropped"),
        }
    }

    fn dispatch_device_session(&mut self, context: Context, message: Message) {
        let unit = &mut self.device_session;
        match message {
            Message::SendComRequest(message) => unit.send_com_request(context, message),
            Message::CommitBatch(message) => unit.commit_batch(context, message),
            Message::SendPacket(message) => unit.send_packet(context, message),
            Message::SecuritySendDone(message) => unit.security_send_done(context, message),
            Message::SecurityRecvDoneComPacket(message) => unit.security_recv_com_packet_done(context, message),
            Message::SecurityRecvDoneComIdRequest(message) => unit.security_recv_com_id_request_done(context, message),
            _ => warn!(message = debug(message), "message dropped"),
        }
    }

    fn dispatch_com_session(&mut self, context: Context, message: Message) {
        let unit = &mut self.com_session;
        match message {
            Message::SendComRequest(message) => unit.send_com_request(context, message),
            Message::SendComRequestDone(message) => unit.send_com_request_done(context, message),
            Message::Timeout(time) => unit.timeout(time),
            Message::ComResponseReceived(message) => unit.com_response_received(message),
            _ => warn!(message = debug(message), "message dropped"),
        }
    }

    fn dispatch_management_session(&mut self, context: Context, message: Message) {
        let unit = &mut self.management_session;
        match message {
            Message::SendMethod(message) => unit.send_method(context, message),
            Message::SendPacketDone(message) => unit.send_packet_done(context, message),
            Message::Timeout(time) => unit.timeout(time),
            Message::PacketReceived(message) => unit.packet_received(context, message),
            _ => warn!(message = debug(message), "message dropped"),
        }
    }

    fn dispatch_session(&mut self, session_id: SessionId, context: Context, message: Message) {
        if let Some(unit) = self.sessions.get_mut(&session_id) {
            match message {
                Message::SendMethod(message) => unit.send_method(context, message),
                Message::CommitBatch(message) => unit.commit_batch(context, message),
                Message::SendPacketDone(message) => unit.send_packet_done(context, message),
                Message::Timeout(time) => unit.timeout(context, time),
                Message::PacketReceived(message) => unit.packet_reveived(context, message),
                Message::Abort(_) => unit.abort(context),
                Message::ReportAborted(message) => unit.report_aborted(context, message),
                _ => drop(Span::current().record("dropped", true)),
            }
        } else {
            warn!(session_id = debug(session_id), "session not found");
        }
    }
}

/// The interface to interact with a running [`Protocol`] stack.
#[derive(Debug, Clone)]
pub struct Controller {
    context: Context,
    connection_changed: async_broadcast::Receiver<ConnectionChanged>,
}

impl Controller {
    /// Perform an remote procedure call using tokenized methods.
    pub fn call(&self, session_id: SessionId, method_tokens: Vec<u8>) -> oneshot::Receiver<MethodResponse> {
        let root_span = Span::current();
        // TODO: remove sensitive data from traces.
        let _span =
            debug_span!("call", session_id = debug(&session_id), method_tokens = debug(&method_tokens)).entered();

        let address = Address::from(session_id);
        let (tx, rx) = oneshot::channel();
        self.context
            .with_root_span(root_span)
            .send(address, Message::SendMethod(SendMethod { method: method_tokens, channel: tx }));
        rx
    }

    /// Notify the protocol stack that a session has been aborted by the device.
    ///
    /// This cleans up the session without sending an EndOfSession, since the
    /// device has already terminated it.
    pub fn report_aborted(&self, session_id: SessionId) {
        let root_span = Span::current();
        let _span = debug_span!("report_aborted", session_id = debug(&session_id)).entered();
        self.context
            .with_root_span(root_span)
            .send(Address::Session(session_id), Message::ReportAborted(ReportAborted));
    }

    /// Send a ComID request to the device.
    pub fn com_id_request(&self, request: ComIdRequest) -> oneshot::Receiver<ComResponse> {
        let root_span = Span::current();
        let _span = debug_span!("com_id_request", request = debug(&request)).entered();

        let address = Address::ComSession;
        let (tx, rx) = oneshot::channel();
        self.context
            .with_root_span(root_span)
            .send(address, Message::SendComRequest(SendComRequest { request, channel: tx }));
        rx
    }

    /// Spawn a new session with the given ID and properties.
    #[cfg(feature = "test-utils")]
    pub fn spawn(&self, session_id: SessionId, properties: Properties) {
        use crate::protocol::message::Spawn;

        let root_span = Span::current();
        let _span = debug_span!("spawn", session_id = debug(&session_id), properties = debug(&properties)).entered();

        let address = Address::Control;
        self.context
            .with_root_span(root_span)
            .send(address, Message::Spawn(Spawn { id: session_id, properties }));
    }

    /// Delete the session with the given ID.
    #[cfg(feature = "test-utils")]
    pub fn delete(&self, session_id: SessionId) {
        use crate::protocol::message::Delete;

        let root_span = Span::current();
        let _span = debug_span!("delete", session_id = debug(&session_id)).entered();

        let address = Address::Control;
        self.context.with_root_span(root_span).send(address, Message::Delete(Delete(session_id)));
    }

    /// Listen to changes in connection properties.
    ///
    /// The protocol manages the properties of the communication with the
    /// remote. When the properties change, an event is emitted on the channel.
    /// This typically only happens once at the beginning of the session, as
    /// it's enough to negotiate properties once. If no event comes on the
    /// channel, it means that the device did not respond to the request to
    /// negotiate properties.
    pub fn connection_changed(&self) -> async_broadcast::Receiver<ConnectionChanged> {
        self.connection_changed.clone()
    }
}

#[derive(Debug, Clone)]
pub struct Context {
    message_queue: async_channel::Sender<DispatchMessage>,
    root_span: Span,
}

impl Context {
    pub fn new(message_queue: async_channel::Sender<DispatchMessage>) -> Self {
        Self { message_queue, root_span: Span::none() }
    }

    pub fn send(&self, address: Address, message: Message) {
        self.message_queue
            .try_send(DispatchMessage { address, context: self.clone(), message })
            .expect("bug: not using on unbounded channel or the channel got closed too early");
    }

    pub fn send_timeout(&self, address: Address, time: Instant, cancel: Option<CancelToken>) {
        let message_queue = self.message_queue.clone();
        let context = self.clone();
        let root_span = context.root_span.clone();
        spawn(
            async move {
                // This check is only necessary for testing with zero timeouts.
                let cancelled = if Instant::now() < time {
                    if let Some(cancel) = cancel {
                        timeout_at(time.clone(), cancel).await.is_ok()
                    } else {
                        sleep_until(time.clone()).await;
                        false
                    }
                } else {
                    false
                };
                if !cancelled {
                    let _ =
                        message_queue.try_send(DispatchMessage { address, context, message: Message::Timeout(time) });
                }
            }
            .instrument(debug_span!(parent: root_span, "timeout").follows_from(Span::current()).clone()),
        );
    }

    pub fn send_future<F>(&self, address: Address, future: F)
    where
        F: Future<Output = Message> + Send + 'static,
    {
        let message_queue = self.message_queue.clone();
        let context = self.clone();
        let root_span = context.root_span.clone();
        spawn(
            async move {
                let message = future.await;
                // The protocol has already been shut down, but that's okay.
                let _ = message_queue.try_send(DispatchMessage { address, context, message });
            }
            .instrument(debug_span!(parent: root_span, "future").follows_from(Span::current()).clone()),
        );
    }

    pub fn with_root_span(&self, root_span: Span) -> Self {
        Self { message_queue: self.message_queue.clone(), root_span }
    }

    #[cfg(test)]
    pub fn mock() -> (Self, async_channel::Receiver<DispatchMessage>) {
        let (tx, rx) = async_channel::unbounded();
        (Self { message_queue: tx, root_span: Span::none() }, rx)
    }
}

#[derive(Debug)]
pub struct DispatchMessage {
    pub address: Address,
    pub context: Context,
    pub message: Message,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Address {
    Control,
    DeviceSession,
    ComSession,
    ManagementSession,
    Session(SessionId),
}

impl From<SessionId> for Address {
    fn from(value: SessionId) -> Self {
        if value == SessionId::MANAGEMENT {
            Self::ManagementSession
        } else {
            Self::Session(value)
        }
    }
}
