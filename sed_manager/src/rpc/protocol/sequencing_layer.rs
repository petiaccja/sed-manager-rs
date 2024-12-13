use tokio::sync::Mutex;

use async_trait::async_trait;

use crate::{messaging::packet::Packet, rpc::error::Error};

use super::packet_layer::PacketLayer;

pub struct SequencingLayer {
    sn_to_send: Mutex<u32>,
    sn_received: Mutex<u32>,
    next_layer: Box<dyn PacketLayer>,
}

impl SequencingLayer {
    pub fn new(next_layer: Box<dyn PacketLayer>) -> Self {
        Self { sn_to_send: 1.into(), sn_received: 0.into(), next_layer }
    }
}

#[async_trait]
impl PacketLayer for SequencingLayer {
    async fn send(&self, packet: Packet) -> Result<(), Error> {
        let mut sn_to_send = self.sn_to_send.lock().await;
        let packet = Packet { sequence_number: *sn_to_send, ..packet };
        self.next_layer.send(packet).await?;
        *sn_to_send += 1;
        Ok(())
    }

    async fn recv(&self) -> Result<Packet, Error> {
        let packet = self.next_layer.recv().await?;
        let mut sn_received = self.sn_received.lock().await;
        if packet.sequence_number == *sn_received + 1 {
            *sn_received = packet.sequence_number;
            Ok(packet)
        } else {
            Err(Error::MissingPacket)
        }
    }

    async fn close(&self) {
        self.next_layer.close().await;
    }

    async fn abort(&self) {
        self.next_layer.abort().await;
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        messaging::packet::{SubPacket, SubPacketKind},
        rpc::protocol::packet_layer::MockPacketLayer,
    };

    use super::*;

    fn make_data() -> Packet {
        Packet {
            payload: vec![SubPacket { kind: SubPacketKind::Data, payload: vec![0; 1].into() }].into(),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn sent_sequence_numbers() {
        let next_layer = Box::new(MockPacketLayer::new());
        let buffering_layer = Arc::new(SequencingLayer::new(next_layer.clone()));

        assert!(buffering_layer.send(make_data()).await.is_ok());
        assert!(buffering_layer.send(make_data()).await.is_ok());
        assert!(buffering_layer.send(make_data()).await.is_ok());
        buffering_layer.close().await;

        let enqueued = next_layer.take_enqueued().await;
        let sequence_numbers: Vec<_> = enqueued.iter().map(|packet| packet.sequence_number).collect();
        assert_eq!(sequence_numbers, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn recv_in_order() {
        let next_layer = Box::new(MockPacketLayer::new());
        let buffering_layer = Arc::new(SequencingLayer::new(next_layer.clone()));

        next_layer.add_dequeue(Packet { sequence_number: 1, ..Default::default() }).await;
        next_layer.add_dequeue(Packet { sequence_number: 2, ..Default::default() }).await;

        assert_eq!(buffering_layer.recv().await.unwrap().sequence_number, 1);
        assert_eq!(buffering_layer.recv().await.unwrap().sequence_number, 2);
        buffering_layer.close().await;
    }

    #[tokio::test]
    async fn recv_missing() {
        let next_layer = Box::new(MockPacketLayer::new());
        let buffering_layer = Arc::new(SequencingLayer::new(next_layer.clone()));

        next_layer.add_dequeue(Packet { sequence_number: 1, ..Default::default() }).await;
        next_layer.add_dequeue(Packet { sequence_number: 3, ..Default::default() }).await;

        assert_eq!(buffering_layer.recv().await.unwrap().sequence_number, 1);
        assert_eq!(buffering_layer.recv().await, Err(Error::MissingPacket));
        buffering_layer.close().await;
    }
}
