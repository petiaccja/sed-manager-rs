use core::mem::drop;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::Arc;
use std::time::{Duration, Instant};

use sed_async_runtime::{sleep_until, spawn};
use sed_device::Device;
use sed_packet::com_id::HandleComIdRequest;
use sed_packet::packet::{COM_PACKET_HEADER_LEN, PACKET_HEADER_LEN, SUB_PACKET_HEADER_LEN};
use sed_spec::methods::Properties;
use tracing::field::Empty;
use tracing::{Instrument, Span, instrument, trace_span};

use crate::protocol::device_session::DeviceSession;
use crate::protocol::message::{ComResponse, Message, MethodResponse, SendComRequest, SendMethod};
use crate::protocol::{
    com_session::ComSession, management_session::ManagementSession, session::Session, session_id::SessionId,
};

const MAX_BUFFER_SIZE: usize = 1048576;

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
    max_methods: 0,
    max_subpackets: 0,
    max_gross_packet_size: MAX_BUFFER_SIZE - COM_PACKET_HEADER_LEN,
    max_packets: 0,
    max_gross_compacket_size: MAX_BUFFER_SIZE,
    max_gross_compacket_response_size: MAX_BUFFER_SIZE,
    max_ind_token_size: MAX_BUFFER_SIZE - COM_PACKET_HEADER_LEN - PACKET_HEADER_LEN - SUB_PACKET_HEADER_LEN,
    max_agg_token_size: MAX_BUFFER_SIZE - COM_PACKET_HEADER_LEN - PACKET_HEADER_LEN - SUB_PACKET_HEADER_LEN,
    continued_tokens: false,
    seq_numbers: false,
    ack_nak: false,
    asynchronous: false,
    buffer_mgmt: false,
    max_retries: 3,
    trans_timeout: Duration::from_secs(15),
    def_trans_timeout: Duration::from_secs(15),
};

/// The full protocol to communicate with the TPer via packets and ComID requests.
#[derive(Debug)]
pub struct Protocol {
    device_session: DeviceSession,
    com_session: ComSession,
    management_session: ManagementSession,
    sessions: HashMap<SessionId, Session>,
    message_queue: async_channel::Receiver<(Address, Message)>,
    context: Context,
}

impl Protocol {
    /// Create a new protocol stack for the `device` on the given ComID and
    /// ComID extension.
    ///
    /// This initializes the protocol stack, but no messages will be delivered
    /// until you call [`run`](Self::run).
    pub fn new(com_id: u16, com_id_ext: u16, device: Arc<dyn Device>) -> Self {
        let (tx, rx) = async_channel::unbounded();
        Self {
            device_session: DeviceSession::new(com_id, com_id_ext, device),
            com_session: ComSession::new(),
            management_session: ManagementSession::new(CAPABILITIES),
            sessions: HashMap::new(),
            message_queue: rx,
            context: Context { message_queue: tx },
        }
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
        while let Ok((address, message)) = self.message_queue.recv().await {
            self.dispatch(address, message);

            // When the sender count is one, `self` is holding the one and only
            // context, meaning no more messages can be received. Internal
            // sessions would hold a context while waiting for an IF command
            // from the device, and external clients would hold a context to
            // issue commands.
            if self.message_queue.sender_count() == 1 {
                break;
            }
        }
    }

    #[instrument]
    fn dispatch(&mut self, address: Address, message: Message) {
        match address {
            Address::Control => self.dispatch_control(message),
            Address::DeviceSession => self.dispatch_device_session(message),
            Address::ComSession => self.dispatch_com_session(message),
            Address::ManagementSession => self.dispatch_management_session(message),
            Address::Session(session_id) => self.dispatch_session(session_id, message),
        }
    }

    #[instrument(fields(dropped = Empty, missing_session = Empty, duplicate_session = Empty))]
    fn dispatch_control(&mut self, message: Message) {
        match message {
            Message::Spawn(message) => match self.sessions.entry(message.id) {
                Entry::Occupied(_) => drop(Span::current().record("dropped", true)),
                Entry::Vacant(entry) => {
                    let _ = entry.insert(Session::new(message.id, message.properties));
                }
            },
            Message::Delete(message) => {
                if self.sessions.remove(&message.0).is_none() {
                    Span::current().record("missing_session", tracing::field::display(message.0));
                }
            }
            _ => drop(Span::current().record("dropped", true)),
        }
    }

    #[instrument(fields(dropped = Empty))]
    fn dispatch_device_session(&mut self, message: Message) {
        let context = self.context.clone();
        let unit = &mut self.device_session;
        match message {
            Message::SendComRequest(message) => unit.send_com_request(context, message),
            Message::CommitBatch(_) => unit.commit_batch(context),
            Message::SendPacket(message) => unit.send_packet(context, message),
            Message::SecuritySendDone(message) => unit.security_send_done(context, message),
            Message::SecurityRecvDoneComPacket(message) => unit.security_recv_com_packet_done(context, message),
            Message::SecurityRecvDoneComIdRequest(message) => unit.security_recv_com_id_request_done(context, message),
            _ => drop(Span::current().record("dropped", true)),
        }
    }

    #[instrument(fields(dropped = Empty))]
    fn dispatch_com_session(&mut self, message: Message) {
        let context = self.context.clone();
        let unit = &mut self.com_session;
        match message {
            Message::SendComRequest(message) => unit.send_com_request(context, message),
            Message::SendComRequestDone(message) => unit.send_com_request_done(context, message),
            Message::Timeout(time) => unit.timeout(time),
            Message::ComResponseReceived(message) => unit.com_response_received(message),
            _ => drop(Span::current().record("dropped", true)),
        }
    }

    #[instrument(fields(dropped = Empty))]
    fn dispatch_management_session(&mut self, message: Message) {
        let context = self.context.clone();
        let unit = &mut self.management_session;
        match message {
            Message::SendMethod(message) => unit.send_method(context, message),
            Message::SendPacketDone(message) => unit.send_packet_done(context, message),
            Message::Timeout(time) => unit.timeout(time),
            Message::PacketReceived(message) => unit.packet_received(context, message),
            _ => drop(Span::current().record("dropped", true)),
        }
    }

    #[instrument(fields(dropped = Empty, missing_session = Empty))]
    fn dispatch_session(&mut self, session_id: SessionId, message: Message) {
        let context = self.context.clone();
        if let Some(unit) = self.sessions.get_mut(&session_id) {
            match message {
                Message::SendMethod(message) => unit.send_method(context, message),
                Message::CommitBatch(_) => unit.commit_batch(context),
                Message::SendPacketDone(message) => unit.send_packet_done(context, message),
                Message::Timeout(time) => unit.timeout(context, time),
                Message::PacketReceived(message) => unit.packet_reveived(context, message),
                Message::Abort(_) => unit.abort(context),
                _ => drop(Span::current().record("dropped", true)),
            }
        } else {
            Span::current().record("missing_session", true);
        }
    }
}

/// The interface to interact with a running [`Protocol`] stack.
#[derive(Debug, Clone)]
pub struct Controller {
    context: Context,
}

impl Controller {
    /// Perform an remote procedure call using tokenized methods.
    pub fn call(
        &self,
        session_id: Option<SessionId>,
        method_tokens: Vec<u8>,
        span: Span,
    ) -> oneshot::Receiver<MethodResponse> {
        let address = match session_id {
            Some(session_id) => Address::from(session_id),
            None => Address::ManagementSession,
        };
        let (tx, rx) = oneshot::channel();
        self.context
            .send(address, Message::SendMethod(SendMethod { method: method_tokens, channel: tx, span }));
        rx
    }

    /// Send a ComID request to the device.
    pub fn com_id_request(&self, request: HandleComIdRequest, span: Span) -> oneshot::Receiver<ComResponse> {
        let address = Address::ComSession;
        let (tx, rx) = oneshot::channel();
        self.context.send(address, Message::SendComRequest(SendComRequest { request, channel: tx, span }));
        rx
    }
}

#[derive(Debug, Clone)]
pub struct Context {
    message_queue: async_channel::Sender<(Address, Message)>,
}

impl Context {
    pub fn send(&self, address: Address, message: Message) {
        self.message_queue
            .try_send((address, message))
            .expect("bug: not using on unbounded channel or the channel got closed too early");
    }

    pub fn send_timeout(&self, address: Address, time: Instant) {
        self.send_future(address, async move {
            sleep_until(time.clone()).instrument(trace_span!("timeout")).await;
            Message::Timeout(time)
        });
    }

    pub fn send_future<F>(&self, address: Address, future: F)
    where
        F: Future<Output = Message> + Send + 'static,
    {
        let message_queue = self.message_queue.clone();
        let span = trace_span!("send_future");
        span.follows_from(Span::current());
        spawn(
            async move {
                let message = future.await;
                // The protocol has already been shut down, but that's okay.
                let _ = message_queue.try_send((address, message));
            }
            .instrument(span),
        );
    }
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
        if value.hsn == 0 && value.tsn == 0 {
            Self::ManagementSession
        } else {
            Self::Session(value)
        }
    }
}
