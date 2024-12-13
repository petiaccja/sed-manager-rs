use async_trait::async_trait;
use tokio::sync::Semaphore;

use crate::{
    messaging::packet::{Packet, SubPacketKind},
    rpc::error::Error,
};

use super::packet_layer::PacketLayer;

pub struct BufferingLayer {
    credit: Semaphore,
    next_layer: Box<dyn PacketLayer>,
}

impl BufferingLayer {
    pub fn new(initial_credit: u32, next_layer: Box<dyn PacketLayer>) -> Self {
        Self { credit: Semaphore::new(initial_credit as usize), next_layer }
    }
}

#[async_trait]
impl PacketLayer for BufferingLayer {
    async fn send(&self, packet: Packet) -> Result<(), Error> {
        let Ok(permit) = self.credit.acquire_many(packet.credit()).await else {
            return Err(Error::AbortedByHost);
        };
        permit.forget();
        self.next_layer.send(packet).await
    }

    async fn recv(&self) -> Result<Packet, Error> {
        let packet = self.next_layer.recv().await?;
        let credit = granted_credit(&packet)?;
        self.credit.add_permits(credit as usize);
        Ok(packet)
    }

    async fn close(&self) {
        self.next_layer.close().await;
        self.credit.close();
    }

    async fn abort(&self) {
        self.next_layer.abort().await;
        self.credit.close();
    }
}

fn granted_credit(packet: &Packet) -> Result<u32, Error> {
    let mut credit: u32 = 0;
    for sub_packet in packet.payload.as_slice() {
        if sub_packet.kind == SubPacketKind::CreditControl {
            let Ok(bytes) = <[u8; 4]>::try_from(sub_packet.payload.as_slice()) else {
                return Err(Error::InvalidCreditControl);
            };
            credit += u32::from_be_bytes(bytes);
        }
    }
    Ok(credit)
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use tokio::time::sleep;

    use super::super::with_copy::with_copy;
    use crate::{messaging::packet::SubPacket, rpc::protocol::packet_layer::MockPacketLayer};

    use super::*;

    fn make_data(credit: u32) -> Packet {
        Packet {
            payload: vec![SubPacket { kind: SubPacketKind::Data, payload: vec![0; credit as usize].into() }].into(),
            ..Default::default()
        }
    }

    fn make_cc(granted: u32) -> Packet {
        Packet {
            payload: vec![SubPacket {
                kind: SubPacketKind::CreditControl,
                payload: Vec::from(granted.to_be_bytes()).into(),
            }]
            .into(),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn block() {
        let next_layer = Box::new(MockPacketLayer::new());
        let buffering_layer = Arc::new(BufferingLayer::new(199, next_layer.clone()));

        let result_task = tokio::spawn(with_copy!(buffering_layer, buffering_layer.send(make_data(200))));
        sleep(Duration::from_millis(50)).await;
        buffering_layer.abort().await;
        let result = result_task.await.unwrap();

        assert_eq!(result, Err(Error::AbortedByHost));
        assert!(next_layer.take_enqueued().await.is_empty());
    }

    #[tokio::test]
    async fn allow() {
        let next_layer = Box::new(MockPacketLayer::new());
        let buffering_layer = Arc::new(BufferingLayer::new(200, next_layer.clone()));

        let result_task = tokio::spawn(with_copy!(buffering_layer, buffering_layer.send(make_data(200))));
        sleep(Duration::from_millis(50)).await;
        buffering_layer.close().await;
        let result = result_task.await.unwrap();

        assert_eq!(result, Ok(()));
        assert_eq!(next_layer.take_enqueued().await.len(), 1);
    }

    #[tokio::test]
    async fn grant_credit() {
        let next_layer = Box::new(MockPacketLayer::new());
        let buffering_layer = Arc::new(BufferingLayer::new(15, next_layer.clone()));

        next_layer.add_dequeue(make_cc(200)).await;

        let _ = buffering_layer.recv().await.unwrap();
        buffering_layer.close().await;

        assert_eq!(buffering_layer.credit.available_permits(), 215);
    }
}
