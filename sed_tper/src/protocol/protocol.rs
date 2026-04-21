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
use tracing::{Instrument, Span, debug, debug_span, instrument, trace_span, warn};

use crate::error::Error;
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
    message_receiver: async_channel::Receiver<(Address, Message)>,
    message_sender: async_channel::WeakSender<(Address, Message)>,
    shutdown: Option<(oneshot::Sender<Result<(), Error>>, Instant)>,
}

impl Protocol {
    /// Create a new protocol stack for the `device` on the given ComID and
    /// ComID extension.
    ///
    /// This initializes the protocol stack, but no messages will be delivered
    /// until you call [`run`](Self::run).
    pub fn new(com_id: u16, com_id_ext: u16, device: Arc<dyn Device>) -> (Self, Controller) {
        let (tx, rx) = async_channel::unbounded();
        let message_sender = tx.clone().downgrade();
        let controller = Controller { context: Context::new(tx) };
        (
            Self {
                device_session: DeviceSession::new(com_id, com_id_ext, device),
                com_session: ComSession::new(CAPABILITIES.def_trans_timeout),
                management_session: ManagementSession::new(CAPABILITIES),
                sessions: HashMap::new(),
                message_receiver: rx,
                message_sender,
                shutdown: None,
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
        loop {
            match self.recv_message().await {
                Ok((address, message)) => self.dispatch(address, message),
                Err(RecvError::Empty) => break,
                Err(RecvError::TimedOut) => {
                    println!("timed out");
                    if let Some((sender, _)) = self.shutdown.take() {
                        let _ = sender.send(Err(Error::TimedOut));
                    }
                    break;
                }
            }

            panic!("THIS THING IS RACY!");
            // With a WeakSender:
            // Someone might drop the last Sender between recv and dispatch,
            // causing the channel to close too early.
            //
            // With ref counting:
            // We might receive the notify sooner than the ref count is decreased.
            // This will get the protocol stuck in the next recv forever.

            // When the sender count is one, `self` is holding the one and only
            // context, meaning no more messages can be received. Internal
            // sessions would hold a context while waiting for an IF command
            // from the device, and external clients would hold a context to
            // issue commands.
            let sender_count = self.message_receiver.sender_count();
            println!("Sender count: {sender_count}");
            if sender_count == 1 {
                break;
            }
            debug!(sender_count = sender_count);
        }
    }

    async fn recv_message(&mut self) -> Result<(Address, Message), RecvError> {
        // if let Some((_, deadline)) = self.shutdown {
        //     timeout_at(deadline, self.message_queue.recv())
        //         .await
        //         .map_err(|_| RecvError::TimedOut)?
        //         .map_err(|_| RecvError::Empty)
        // } else {
        self.message_receiver.recv().await.map_err(|_| RecvError::Empty)
        //}
    }

    fn dispatch(&mut self, address: Address, message: Message) {
        println!("Message: {message:?} -> {address:?}");
        match address {
            Address::Control => self.dispatch_control(message),
            Address::DeviceSession => self.dispatch_device_session(message),
            Address::ComSession => self.dispatch_com_session(message),
            Address::ManagementSession => self.dispatch_management_session(message),
            Address::Session(session_id) => self.dispatch_session(session_id, message),
        }
    }

    fn dispatch_control(&mut self, message: Message) {
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
            Message::Shutdown(sender, deadline) => {
                self.shutdown = Some((sender, deadline));
            }
            _ => warn!(message = debug(message), "message dropped"),
        }
    }

    fn dispatch_device_session(&mut self, message: Message) {
        let context = self.context.clone();
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

    fn dispatch_com_session(&mut self, message: Message) {
        let context = self.context.clone();
        let unit = &mut self.com_session;
        match message {
            Message::SendComRequest(message) => unit.send_com_request(context, message),
            Message::SendComRequestDone(message) => unit.send_com_request_done(context, message),
            Message::Timeout(time) => unit.timeout(time),
            Message::ComResponseReceived(message) => unit.com_response_received(message),
            _ => warn!(message = debug(message), "message dropped"),
        }
    }

    fn dispatch_management_session(&mut self, message: Message) {
        let context = self.context.clone();
        let unit = &mut self.management_session;
        match message {
            Message::SendMethod(message) => unit.send_method(context, message),
            Message::SendPacketDone(message) => unit.send_packet_done(context, message),
            Message::Timeout(time) => unit.timeout(time),
            Message::PacketReceived(message) => unit.packet_received(context, message),
            _ => warn!(message = debug(message), "message dropped"),
        }
    }

    fn dispatch_session(&mut self, session_id: SessionId, message: Message) {
        let context = self.context.clone();
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

enum RecvError {
    Empty,
    TimedOut,
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

    #[instrument(level = "debug")]
    pub async fn shutdown(self, timeout: Duration) -> Result<(), Error> {
        let (tx, rx) = oneshot::channel();
        let deadline = Instant::now() + timeout;
        self.context.send(Address::Control, Message::Shutdown(tx, deadline));
        drop(self);
        rx.await.unwrap_or(Ok(()))
    }
}

#[derive(Debug, Clone)]
pub struct Context {
    message_queue: async_channel::Sender<(Address, Message)>,
    notify_drop: bool,
}

impl Context {
    pub fn new(message_queue: async_channel::Sender<(Address, Message)>) -> Self {
        Self { message_queue, notify_drop: true }
    }

    #[instrument(level = "debug")]
    pub fn send(&self, address: Address, message: Message) {
        self.message_queue
            .try_send((address, message))
            .expect("bug: not using on unbounded channel or the channel got closed too early");
    }

    #[instrument(level = "debug", skip(cancel))]
    pub fn send_timeout(&self, address: Address, time: Instant, cancel: Option<CancelToken>) {
        let message_queue = self.message_queue.clone();
        spawn(
            async move {
                // This check is only necessary for testing with zero timeouts.
                let cancelled = if Instant::now() < time {
                    if let Some(cancel) = cancel {
                        timeout_at(time.clone(), cancel).instrument(trace_span!("timeout")).await.is_ok()
                    } else {
                        sleep_until(time.clone()).await;
                        false
                    }
                } else {
                    false
                };
                if !cancelled {
                    let _ = message_queue.try_send((address, Message::Timeout(time)));
                }
            }
            .in_current_span(),
        );
    }

    #[instrument(level = "debug", skip(future))]
    pub fn send_future<F>(&self, address: Address, future: F)
    where
        F: Future<Output = Message> + Send + 'static,
    {
        let message_queue = self.message_queue.clone();
        spawn(
            async move {
                let message = future.await;
                // The protocol has already been shut down, but that's okay.
                let _ = message_queue.try_send((address, message));
            }
            .in_current_span(),
        );
    }

    #[cfg(test)]
    pub fn mock() -> (Self, async_channel::Receiver<(Address, Message)>) {
        let (tx, rx) = async_channel::unbounded();
        (Self { message_queue: tx, notify_drop: false }, rx)
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        // When the last outside context is dropped, we need to wake up the run
        // task or otherwise it would be waiting for a message forever. It would
        // be better to send a notification unconditionally, but contexts are
        // created and dropped a lot, so it may not be the best for performance.
        if self.notify_drop {
            let _span = debug_span!("context_drop").entered();
            if self.message_queue.sender_count() <= 2 {
                let _span = debug_span!("notify").entered();
                if self.message_queue.try_send((Address::Control, Message::ContextDropped)).is_ok() {
                    let _span = debug_span!("notify_done").entered();
                }
            }
        }
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
