use async_trait::async_trait;
use std::collections::VecDeque as Queue;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::traits::InterfaceLayer;
use super::traits::PacketLayer;
use crate::messaging::packet::{ComPacket, Packet};
use crate::rpc::error::Error;
use crate::rpc::properties::Properties;

pub struct ComPacketBundler {
    com_id: u16,
    com_id_ext: u16,
    next_layer: Arc<dyn InterfaceLayer>,
    properties: Properties,
    queue: Mutex<Queue<Packet>>,
}

impl ComPacketBundler {
    pub fn new(com_id: u16, com_id_ext: u16, interface_layer: Arc<dyn InterfaceLayer>, properties: Properties) -> Self {
        Self { com_id, com_id_ext, next_layer: interface_layer, properties: properties, queue: Queue::new().into() }
    }
}

#[async_trait]
impl PacketLayer for ComPacketBundler {
    async fn send(&self, packet: Packet) -> Result<(), Error> {
        let com_packet = ComPacket {
            com_id: self.com_id,
            com_id_ext: self.com_id_ext,
            outstanding_data: 0,
            min_transfer: 0,
            payload: vec![packet].into(),
        };
        self.next_layer.send_com_packet(com_packet).await
    }

    async fn recv(&self) -> Result<Packet, Error> {
        let mut queue = self.queue.lock().await;
        loop {
            if let Some(packet) = queue.pop_front() {
                break Ok(packet);
            } else {
                let com_packet = self.next_layer.recv_com_packet().await?;
                for packet in com_packet.payload.into_vec() {
                    queue.push_back(packet);
                }
            }
        }
    }

    async fn close(&self) {
        self.next_layer.close().await;
    }

    async fn abort(&self) {
        self.next_layer.abort().await;
    }
}
