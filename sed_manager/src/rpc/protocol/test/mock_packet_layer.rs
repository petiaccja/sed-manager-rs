use async_trait::async_trait;
use std::{collections::VecDeque as Queue, sync::Arc};
use tokio::sync::Mutex;

use crate::messaging::packet::Packet;
use crate::rpc::pipeline::{connect, BufferedSend, Connect, PushInput, PushOutput, Receive};
use crate::rpc::protocol::PacketLayer;
use crate::rpc::Error;

#[derive(Clone)]
pub struct MockPacketLayer {
    enqueue_tx: Arc<Mutex<PushOutput<Packet>>>,
    enqueue_rx: Arc<Mutex<PushInput<Packet>>>,
    dequeue_tx: Arc<Mutex<PushOutput<Packet>>>,
    dequeue_rx: Arc<Mutex<PushInput<Packet>>>,
}

impl MockPacketLayer {
    pub fn new() -> Self {
        let (mut enqueue_tx, mut enqueue_rx) = (PushOutput::new(), PushInput::new());
        let (mut dequeue_tx, mut dequeue_rx) = (PushOutput::new(), PushInput::new());
        connect(&mut enqueue_tx, &mut enqueue_rx);
        connect(&mut dequeue_tx, &mut dequeue_rx);
        Self {
            enqueue_tx: Arc::new(Mutex::new(enqueue_tx)),
            enqueue_rx: Arc::new(Mutex::new(enqueue_rx)),
            dequeue_tx: Arc::new(Mutex::new(dequeue_tx)),
            dequeue_rx: Arc::new(Mutex::new(dequeue_rx)),
        }
    }

    pub async fn take_enqueued(&self) -> Queue<Packet> {
        let mut queue = Queue::new();
        let mut enqueue_rx = self.enqueue_rx.lock().await;
        while let Some(packet) = enqueue_rx.try_recv() {
            queue.push_back(packet);
        }
        queue
    }

    pub async fn wait_enqueued(&self) -> Option<Packet> {
        let mut enqueue_rx = self.enqueue_rx.lock().await;
        enqueue_rx.recv().await
    }

    pub async fn add_dequeue(&self, packet: Packet) {
        let _ = self.dequeue_tx.lock().await.send(packet);
    }
}

#[async_trait]
impl PacketLayer for MockPacketLayer {
    async fn send(&self, packet: Packet) -> Result<(), Error> {
        let _ = self.enqueue_tx.lock().await.send(packet);
        Ok(())
    }

    async fn recv(&self) -> Result<Packet, Error> {
        if let Some(packet) = self.dequeue_rx.lock().await.recv().await {
            Ok(packet)
        } else {
            Err(Error::Closed)
        }
    }

    async fn close(&self) {
        self.dequeue_tx.lock().await.disconnect();
    }

    async fn abort(&self) {
        self.dequeue_tx.lock().await.disconnect();
    }
}
