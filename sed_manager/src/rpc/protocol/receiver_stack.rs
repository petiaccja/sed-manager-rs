use std::collections::{HashMap, VecDeque};

use tokio::sync::oneshot;

use crate::messaging::com_id::HandleComIdResponse;
use crate::messaging::packet::ComPacket;
use crate::rpc::error::{ErrorEvent, ErrorEventExt as _};
use crate::rpc::{Error, PackagedMethod, Properties};

use super::packet_receiver::PacketReceiver;
use super::session_identifier::SessionIdentifier;
use super::timeout::Timeout;

type PacketResponse = Result<PackagedMethod, Error>;
type ComIdResponse = Result<HandleComIdResponse, Error>;
type PacketPromise = oneshot::Sender<PacketResponse>;
type ComIdPromise = oneshot::Sender<ComIdResponse>;

pub struct ReceiverStack {
    packet_receivers: HashMap<SessionIdentifier, PacketReceiver>,
    com_id_promises: VecDeque<ComIdPromise>,
    com_id_timeout: Timeout<HandleComIdResponse>,
}

impl ReceiverStack {
    pub fn new(properties: Properties) -> Self {
        Self {
            packet_receivers: HashMap::new(),
            com_id_promises: VecDeque::new(),
            com_id_timeout: Timeout::new(properties),
        }
    }

    pub fn insert_session(&mut self, session: SessionIdentifier, properties: Properties) {
        let entry = self.packet_receivers.entry(session);
        entry.or_insert_with(|| PacketReceiver::new(properties));
    }

    pub fn remove_session(&mut self, session: SessionIdentifier) -> bool {
        if let Some(sender) = self.packet_receivers.get(&session) {
            if sender.has_pending() {
                false
            } else {
                drop(self.packet_receivers.remove(&session));
                true
            }
        } else {
            true
        }
    }

    pub fn abort_session(&mut self, session: SessionIdentifier) {
        drop(self.packet_receivers.remove(&session));
    }

    pub fn enqueue_packet_response(&mut self, com_packet: ComPacket) {
        for packet in com_packet.payload.into_vec() {
            let session = SessionIdentifier::from(&packet);
            if let Some(receiver) = self.packet_receivers.get_mut(&session) {
                receiver.enqueue_packet(packet);
            }
        }
    }

    pub fn enqueue_packet_promise(&mut self, session: SessionIdentifier, promise: PacketPromise) {
        if let Some(receiver) = self.packet_receivers.get_mut(&session) {
            receiver.enqueue_promise(promise);
        }
    }

    pub fn enqueue_com_id_response(&mut self, response: HandleComIdResponse) {
        self.com_id_timeout.enqueue(Ok(response));
    }

    pub fn enqueue_com_id_promise(&mut self, promise: ComIdPromise) {
        self.com_id_promises.push_back(promise);
    }

    pub fn poll_method(&mut self) -> Option<(PacketPromise, PacketResponse)> {
        for receiver in self.packet_receivers.values_mut() {
            if let Some(result) = receiver.poll() {
                return Some(result);
            }
        }
        None
    }

    pub fn poll_com_id(&mut self) -> Option<(ComIdPromise, Result<HandleComIdResponse, Error>)> {
        if !self.com_id_promises.is_empty() {
            if let Some(result) = self.com_id_timeout.poll() {
                Some((self.com_id_promises.pop_front().unwrap(), result))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn has_pending(&self) -> bool {
        self.packet_receivers.iter().any(|s| s.1.has_pending()) || !self.com_id_promises.is_empty()
    }
}

impl Drop for ReceiverStack {
    fn drop(&mut self) {
        for promise in std::mem::replace(&mut self.com_id_promises, VecDeque::new()) {
            let _ = promise.send(Err(ErrorEvent::Aborted.while_receiving()));
        }
    }
}
