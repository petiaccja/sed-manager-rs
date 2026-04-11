use core::mem::drop;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::Arc;
use std::time::Instant;

use sed_async_runtime::{sleep_until, spawn};
use sed_device::Device;
use tracing::field::Empty;
use tracing::{Instrument, Span, instrument, trace_span};

use crate::protocol::device_session::DeviceSession;
use crate::protocol::message::Message;
use crate::protocol::{
    com_session::ComSession, management_session::ManagementSession, session::Session, session_id::SessionId,
};

#[derive(Debug)]
pub struct Protocol {
    device_session: DeviceSession,
    com_session: ComSession,
    management_session: ManagementSession,
    sessions: HashMap<SessionId, Session>,
    message_queue: async_channel::Receiver<(Address, Message)>,
    message_queue_sender: async_channel::Sender<(Address, Message)>,
}

impl Protocol {
    pub fn new(com_id: u16, com_id_ext: u16, device: Arc<dyn Device>) -> Self {
        let (tx, rx) = async_channel::unbounded();
        Self {
            device_session: DeviceSession::new(com_id, com_id_ext, device),
            com_session: ComSession::new(),
            management_session: ManagementSession::new(),
            sessions: HashMap::new(),
            message_queue: rx,
            message_queue_sender: tx,
        }
    }

    pub fn dispatch_all_queued(&mut self) {
        while let Ok((address, message)) = self.message_queue.try_recv() {
            self.dispatch(address, message);
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
            Message::Spawn(message) => match self.sessions.entry(message.0) {
                Entry::Occupied(_) => drop(Span::current().record("dropped", true)),
                Entry::Vacant(entry) => {
                    let _ = entry.insert(Session::new(message.0, self.management_session.properties()));
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
        let context = self.context();
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
        let context = self.context();
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
        let context = self.context();
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
        let context = self.context();
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

    fn context(&self) -> Context {
        Context { message_queue: self.message_queue_sender.clone() }
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
            sleep_until(time.clone()).await;
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
