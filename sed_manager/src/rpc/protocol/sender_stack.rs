use std::collections::{HashMap, VecDeque};

use crate::messaging::com_id::{HandleComIdRequest, HandleComIdResponse};
use crate::messaging::packet::{ComPacket, Packet};
use crate::rpc::error::{ErrorEvent, ErrorEventExt as _};
use crate::rpc::{Error, PackagedMethod, Properties};

use super::packet_sender::PacketSender;
use super::session_identifier::SessionIdentifier;
use super::tracked::Tracked;

type PacketResponse = Result<PackagedMethod, Error>;
type ComIdResponse = Result<HandleComIdResponse, Error>;

pub struct SenderStack {
    packet_senders: HashMap<SessionIdentifier, PacketSender>,
    com_id_buffer: VecDeque<Tracked<HandleComIdRequest, ComIdResponse>>,
    bundler: Bundler,
}

struct Bundler {
    com_id: u16,
    com_id_ext: u16,
    #[allow(unused)]
    properties: Properties, // Needed for packet limits, which is not implemented yet.
    buffer: VecDeque<Tracked<Packet, PacketResponse>>,
}

impl SenderStack {
    pub fn new(com_id: u16, com_id_ext: u16, properties: Properties) -> Self {
        Self {
            packet_senders: HashMap::new(),
            com_id_buffer: VecDeque::new(),
            bundler: Bundler::new(com_id, com_id_ext, properties),
        }
    }

    pub fn insert_session(&mut self, session: SessionIdentifier, properties: Properties) {
        let entry = self.packet_senders.entry(session);
        entry.or_insert_with(|| PacketSender::new(session, properties));
    }

    pub fn remove_session(&mut self, session: SessionIdentifier) -> bool {
        if let Some(sender) = self.packet_senders.get(&session) {
            if sender.has_pending() {
                false
            } else {
                drop(self.packet_senders.remove(&session));
                true
            }
        } else {
            true
        }
    }

    pub fn abort_session(&mut self, session: SessionIdentifier) {
        drop(self.packet_senders.remove(&session));
    }

    pub fn enqueue_method(&mut self, session: SessionIdentifier, method: Tracked<PackagedMethod, PacketResponse>) {
        if let Some(sender) = self.packet_senders.get_mut(&session) {
            sender.enqueue(method);
        } else {
            method.close(Err(ErrorEvent::Closed.while_sending()));
        }
    }

    pub fn enqueue_com_id(&mut self, request: Tracked<HandleComIdRequest, ComIdResponse>) {
        self.com_id_buffer.push_back(request);
    }

    pub fn poll_packet(&mut self) -> Option<(ComPacket, Vec<Tracked<SessionIdentifier, PacketResponse>>)> {
        for sender in self.packet_senders.values_mut() {
            while let Some(item) = sender.poll() {
                self.bundler.enqueue(item);
            }
        }
        self.bundler.poll()
    }

    pub fn poll_com_id(&mut self) -> Option<Tracked<HandleComIdRequest, ComIdResponse>> {
        self.com_id_buffer.pop_front()
    }

    pub fn has_pending(&self) -> bool {
        self.packet_senders.iter().any(|s| s.1.has_pending())
    }
}

impl Bundler {
    pub fn new(com_id: u16, com_id_ext: u16, properties: Properties) -> Self {
        Self { com_id, com_id_ext, properties, buffer: VecDeque::new() }
    }

    pub fn enqueue(&mut self, packet: Tracked<Packet, PacketResponse>) {
        self.buffer.push_back(packet);
    }

    pub fn poll(&mut self) -> Option<(ComPacket, Vec<Tracked<SessionIdentifier, PacketResponse>>)> {
        if let Some(Tracked { item, promises }) = self.buffer.pop_front() {
            let session = SessionIdentifier::from(&item);
            let com_packet = ComPacket {
                com_id: self.com_id,
                com_id_ext: self.com_id_ext,
                payload: vec![item].into(),
                ..Default::default()
            };
            Some((com_packet, vec![Tracked::new(session, promises)]))
        } else {
            None
        }
    }
}

impl Drop for SenderStack {
    fn drop(&mut self) {
        for tracked in core::mem::replace(&mut self.com_id_buffer, VecDeque::new()) {
            tracked.close(Err(ErrorEvent::Aborted.while_sending()));
        }
    }
}
