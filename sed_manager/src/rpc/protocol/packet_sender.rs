use std::collections::VecDeque;

use crate::messaging::packet::{Packet, SubPacket, SubPacketKind, PACKET_HEADER_LEN, SUB_PACKET_HEADER_LEN};
use crate::messaging::token::SerializeTokens;
use crate::rpc::{Error, PackagedMethod, Properties};
use crate::serialization::vec_without_len::VecWithoutLen;
use crate::serialization::SerializeBinary;

use super::session_identifier::SessionIdentifier;
use super::tracked::Tracked;

type Response = Result<PackagedMethod, Error>;

pub struct PacketSender {
    session: SessionIdentifier,
    properties: Properties,
    serializer: Serializer,
    bundler: Bundler,
    labeler: Labeler,
}

struct Serializer {
    properties: Properties,
    buffer: VecDeque<Tracked<PackagedMethod, Response>>,
}

struct Bundler {
    properties: Properties,
    buffer: VecDeque<Tracked<Packet, Response>>,
}

struct Labeler {
    session: SessionIdentifier,
    buffer: VecDeque<Tracked<Packet, Response>>,
}

impl PacketSender {
    pub fn new(session: SessionIdentifier, properties: Properties) -> Self {
        Self {
            session,
            properties: properties.clone(),
            serializer: Serializer::new(properties.clone()),
            bundler: Bundler::new(properties),
            labeler: Labeler::new(session),
        }
    }

    pub fn enqueue(&mut self, method: Tracked<PackagedMethod, Response>) {
        self.serializer.enqueue(method)
    }

    pub fn has_pending(&self) -> bool {
        self.serializer.has_pending() || self.bundler.has_pending() || self.labeler.has_pending()
    }

    pub fn poll(&mut self) -> Option<Tracked<Packet, Response>> {
        self.process();
        self.labeler.poll()
    }

    fn process(&mut self) {
        while let Some(packet) = self.serializer.poll() {
            self.bundler.enqueue(packet);
        }
        while let Some(packet) = self.bundler.poll() {
            self.labeler.enqueue(packet);
        }
    }
}

impl Serializer {
    pub fn new(properties: Properties) -> Self {
        Self { properties, buffer: VecDeque::new() }
    }

    pub fn enqueue(&mut self, method: Tracked<PackagedMethod, Response>) {
        self.buffer.push_back(method);
    }

    pub fn has_pending(&self) -> bool {
        !self.buffer.is_empty()
    }

    pub fn poll(&mut self) -> Option<Tracked<Packet, Response>> {
        while let Some(method) = self.buffer.pop_front() {
            if let Some(packet) = self.try_serialize(method) {
                return Some(packet);
            }
        }
        None
    }

    fn try_serialize(&self, method: Tracked<PackagedMethod, Response>) -> Option<Tracked<Packet, Response>> {
        let tokenized = method.try_map(|value| value.to_tokens().map_err(|err| Err(Error::TokenizationFailed(err))))?;
        let serialized = tokenized.try_map(|value| {
            VecWithoutLen::from(value).to_bytes().map_err(|err| Err(Error::SerializationFailed(err)))
        })?;
        serialized.try_map(|value| {
            let limit = self.properties.max_gross_packet_size - PACKET_HEADER_LEN - SUB_PACKET_HEADER_LEN;
            if value.len() <= limit {
                let sub_packet = SubPacket { kind: SubPacketKind::Data, payload: value.into() };
                Ok(Packet { payload: vec![sub_packet].into(), ..Default::default() })
            } else {
                Err(Err(Error::MethodTooLarge))
            }
        })
    }
}

impl Bundler {
    pub fn new(properties: Properties) -> Self {
        Self { properties, buffer: VecDeque::new() }
    }

    pub fn enqueue(&mut self, packet: Tracked<Packet, Response>) {
        self.buffer.push_back(packet);
    }

    pub fn has_pending(&self) -> bool {
        !self.buffer.is_empty()
    }

    pub fn poll(&mut self) -> Option<Tracked<Packet, Response>> {
        self.buffer.pop_front()
    }
}

impl Labeler {
    pub fn new(session: SessionIdentifier) -> Self {
        Self { session, buffer: VecDeque::new() }
    }

    pub fn enqueue(&mut self, packet: Tracked<Packet, Response>) {
        self.buffer.push_back(packet);
    }

    pub fn has_pending(&self) -> bool {
        !self.buffer.is_empty()
    }

    pub fn poll(&mut self) -> Option<Tracked<Packet, Response>> {
        self.buffer.pop_front().map(|tracked| {
            tracked.map(|packet| Packet {
                tper_session_number: self.session.tsn,
                host_session_number: self.session.hsn,
                ..packet
            })
        })
    }
}

impl Drop for PacketSender {
    fn drop(&mut self) {
        while let Some(tracked) = self.poll() {
            tracked.close(Err(Error::AbortedByHost));
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::oneshot;

    use crate::messaging::token::Tag;

    use super::*;

    #[test]
    fn serializer_empty() {
        let mut serializer = Serializer::new(Properties::ASSUMED);
        assert!(!serializer.has_pending());
        assert!(serializer.poll().is_none());
    }

    #[test]
    fn serializer_some() {
        let mut serializer = Serializer::new(Properties::ASSUMED);
        let (tx, _rx) = oneshot::channel();
        serializer.enqueue(Tracked { item: PackagedMethod::EndOfSession, promises: vec![tx] });
        assert!(serializer.has_pending());
        if let Some(Tracked { item, promises }) = serializer.poll() {
            assert_eq!(item.payload[0].payload[0], Tag::EndOfSession as u8);
            assert_eq!(promises.len(), 1);
        } else {
            assert!(false, "expected Some for poll")
        }
        assert!(!serializer.has_pending());
    }

    #[test]
    fn bundler_empty() {
        let mut bundler = Bundler::new(Properties::ASSUMED);
        assert!(!bundler.has_pending());
        assert!(bundler.poll().is_none());
    }

    #[test]
    fn bundler_some() {
        let mut bundler = Bundler::new(Properties::ASSUMED);
        let (tx, _rx) = oneshot::channel();
        bundler.enqueue(Tracked { item: Packet::default(), promises: vec![tx] });
        assert!(bundler.has_pending());
        if let Some(Tracked { item, promises }) = bundler.poll() {
            assert_eq!(item, Packet::default());
            assert_eq!(promises.len(), 1);
        } else {
            assert!(false, "expected Some for poll")
        }
        assert!(!bundler.has_pending());
    }

    #[test]
    fn labeler_empty() {
        let mut labeler = Labeler::new(SessionIdentifier { hsn: 100, tsn: 200 });
        assert!(!labeler.has_pending());
        assert!(labeler.poll().is_none());
    }

    #[test]
    fn labeler_some() {
        let mut labeler = Labeler::new(SessionIdentifier { hsn: 100, tsn: 200 });
        let (tx, _rx) = oneshot::channel();
        labeler.enqueue(Tracked { item: Packet::default(), promises: vec![tx] });
        assert!(labeler.has_pending());
        if let Some(Tracked { item, promises }) = labeler.poll() {
            assert_eq!(item, Packet { host_session_number: 100, tper_session_number: 200, ..Default::default() });
            assert_eq!(promises.len(), 1);
        } else {
            assert!(false, "expected Some for poll")
        }
        assert!(!labeler.has_pending());
    }

    #[test]
    fn packet_sender_empty() {
        let mut sender = PacketSender::new(SessionIdentifier { hsn: 100, tsn: 200 }, Properties::ASSUMED);
        assert!(!sender.has_pending());
        assert!(sender.poll().is_none());
    }

    #[test]
    fn packet_sender_some() {
        let mut sender = PacketSender::new(SessionIdentifier { hsn: 100, tsn: 200 }, Properties::ASSUMED);
        let (tx, _rx) = oneshot::channel();
        sender.enqueue(Tracked { item: PackagedMethod::EndOfSession, promises: vec![tx] });
        assert!(sender.has_pending());
        if let Some(Tracked { item, promises }) = sender.poll() {
            assert_eq!(item.host_session_number, 100);
            assert_eq!(item.tper_session_number, 200);
            assert_eq!(item.payload[0].payload[0], Tag::EndOfSession as u8);
            assert_eq!(promises.len(), 1);
        } else {
            assert!(false, "expected Some for poll")
        }
        assert!(!sender.has_pending());
    }
}
