use core::mem::drop;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::Arc;
use std::time::{Duration, Instant};

use sed_async_runtime::{CancelToken, sleep_until, spawn, timeout_at};
use sed_device::Device;
use sed_packet::com_id::HandleComIdRequest;
use sed_packet::packet::{COM_PACKET_HEADER_LEN, PACKET_HEADER_LEN, SUB_PACKET_HEADER_LEN};
use sed_spec::methods::Properties;
use tracing::{Instrument, Span, debug_span, instrument, warn};

use crate::protocol::device_session::DeviceSession;
use crate::protocol::message::{ComResponse, Message, MethodResponse, SendComRequest, SendMethod};
use crate::protocol::{
    com_session::ComSession, management_session::ManagementSession, session::Session, session_id::SessionId,
};

const MAX_BUFFER_SIZE: usize = 1048576;
#[cfg(not(test))]
const DEFAULT_TRANS_TIMEOUT: Duration = Duration::from_secs(1);
#[cfg(test)]
const DEFAULT_TRANS_TIMEOUT: Duration = Duration::from_secs(15);

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
    trans_timeout: DEFAULT_TRANS_TIMEOUT,
    def_trans_timeout: DEFAULT_TRANS_TIMEOUT,
};

/// The full protocol to communicate with the TPer via packets and ComID requests.
#[derive(Debug)]
pub struct Protocol {
    device_session: DeviceSession,
    com_session: ComSession,
    management_session: ManagementSession,
    sessions: HashMap<SessionId, Session>,
    message_receiver: async_channel::Receiver<(Context, Address, Message)>,
}

impl Protocol {
    /// Create a new protocol stack for the `device` on the given ComID and
    /// ComID extension.
    ///
    /// This initializes the protocol stack, but no messages will be delivered
    /// until you call [`run`](Self::run).
    pub fn new(com_id: u16, com_id_ext: u16, device: Arc<dyn Device>) -> (Self, Controller) {
        let (tx, rx) = async_channel::unbounded();
        let controller = Controller { context: Context::new(tx) };
        (
            Self {
                device_session: DeviceSession::new(com_id, com_id_ext, device),
                com_session: ComSession::new(CAPABILITIES.def_trans_timeout),
                management_session: ManagementSession::new(CAPABILITIES),
                sessions: HashMap::new(),
                message_receiver: rx,
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
    #[instrument(level = "debug")]
    pub async fn run(mut self) {
        while let Ok((context, address, message)) = self.message_receiver.recv().await {
            self.dispatch(context, address, message);
        }
    }

    fn dispatch(&mut self, context: Context, address: Address, message: Message) {
        match address {
            Address::Control => self.dispatch_control(context, message),
            Address::DeviceSession => self.dispatch_device_session(context, message),
            Address::ComSession => self.dispatch_com_session(context, message),
            Address::ManagementSession => self.dispatch_management_session(context, message),
            Address::Session(session_id) => self.dispatch_session(context, session_id, message),
        }
    }

    fn dispatch_control(&mut self, _context: Context, message: Message) {
        match message {
            Message::Spawn(message) => match self.sessions.entry(message.id) {
                Entry::Occupied(_) => warn!(message = debug(message), "message dropped"),
                Entry::Vacant(entry) => {
                    let _ = entry.insert(Session::new(message.id, message.properties));
                }
            },
            Message::Delete(message) => {
                if self.sessions.remove(&message.0).is_none() {
                    warn!(session_id = debug(&message.0), "session not found")
                }
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

    fn dispatch_session(&mut self, context: Context, session_id: SessionId, message: Message) {
        if let Some(unit) = self.sessions.get_mut(&session_id) {
            match message {
                Message::SendMethod(message) => unit.send_method(context, message),
                Message::CommitBatch(message) => unit.commit_batch(context, message),
                Message::SendPacketDone(message) => unit.send_packet_done(context, message),
                Message::Timeout(time) => unit.timeout(context, time),
                Message::PacketReceived(message) => unit.packet_reveived(context, message),
                Message::Abort(_) => unit.abort(context),
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
}

impl Controller {
    /// Perform an remote procedure call using tokenized methods.
    #[instrument(level = "debug")]
    pub fn call(&self, session_id: Option<SessionId>, method_tokens: Vec<u8>) -> oneshot::Receiver<MethodResponse> {
        let address = match session_id {
            Some(session_id) => Address::from(session_id),
            None => Address::ManagementSession,
        };
        let (tx, rx) = oneshot::channel();
        self.context.send(
            address,
            Message::SendMethod(SendMethod { method: method_tokens, channel: tx, span: Span::current() }),
        );
        rx
    }

    /// Send a ComID request to the device.
    #[instrument(level = "debug")]
    pub fn com_id_request(&self, request: HandleComIdRequest) -> oneshot::Receiver<ComResponse> {
        let address = Address::ComSession;
        let (tx, rx) = oneshot::channel();
        self.context
            .send(address, Message::SendComRequest(SendComRequest { request, channel: tx, span: Span::current() }));
        rx
    }
}

#[derive(Debug, Clone)]
pub struct Context {
    message_queue: async_channel::Sender<(Context, Address, Message)>,
}

impl Context {
    pub fn new(message_queue: async_channel::Sender<(Context, Address, Message)>) -> Self {
        Self { message_queue }
    }

    #[instrument(level = "debug")]
    pub fn send(&self, address: Address, message: Message) {
        self.message_queue
            .try_send((self.clone(), address, message))
            .expect("bug: not using on unbounded channel or the channel got closed too early");
    }

    #[instrument(level = "debug", skip(cancel))]
    pub fn send_timeout(&self, address: Address, time: Instant, cancel: Option<CancelToken>) {
        let message_queue = self.message_queue.clone();
        let context = self.clone();
        let timeout_span = debug_span!("timeout");
        timeout_span.follows_from(Span::current());
        spawn(async move {
            // This check is only necessary for testing with zero timeouts.
            let cancelled = if Instant::now() < time {
                if let Some(cancel) = cancel {
                    timeout_at(time.clone(), cancel).instrument(timeout_span).await.is_ok()
                } else {
                    sleep_until(time.clone()).instrument(timeout_span).await;
                    false
                }
            } else {
                false
            };
            if !cancelled {
                let _ = message_queue.try_send((context, address, Message::Timeout(time)));
            }
        });
    }

    #[instrument(level = "debug", skip(future))]
    pub fn send_future<F>(&self, address: Address, future: F)
    where
        F: Future<Output = Message> + Send + 'static,
    {
        let message_queue = self.message_queue.clone();
        let context = self.clone();
        spawn(
            async move {
                let message = future.await;
                // The protocol has already been shut down, but that's okay.
                let _ = message_queue.try_send((context, address, message));
            }
            .in_current_span(),
        );
    }

    #[cfg(test)]
    pub fn mock() -> (Self, async_channel::Receiver<(Context, Address, Message)>) {
        let (tx, rx) = async_channel::unbounded();
        (Self { message_queue: tx }, rx)
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
