use async_trait::async_trait;
use std::ops::Deref;
use std::{collections::VecDeque as Queue, sync::Arc};
use tokio::select;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

use crate::messaging::packet::Packet;
use crate::rpc::protocol::PacketLayer;
use crate::rpc::{Error, Properties};

#[derive(Clone)]
pub struct MockPacketLayer {
    enqueue_tx: Arc<Mutex<Option<mpsc::UnboundedSender<Packet>>>>,
    enqueue_rx: Arc<Mutex<mpsc::UnboundedReceiver<Packet>>>,
    dequeue_tx: Arc<Mutex<Option<mpsc::UnboundedSender<Packet>>>>,
    dequeue_rx: Arc<Mutex<mpsc::UnboundedReceiver<Packet>>>,
    properties: Properties,
}

impl MockPacketLayer {
    pub fn new(properties: Properties) -> Self {
        let (enqueue_tx, enqueue_rx) = mpsc::unbounded_channel();
        let (dequeue_tx, dequeue_rx) = mpsc::unbounded_channel();
        Self {
            enqueue_tx: Arc::new(Mutex::new(Some(enqueue_tx))),
            enqueue_rx: Arc::new(Mutex::new(enqueue_rx)),
            dequeue_tx: Arc::new(Mutex::new(Some(dequeue_tx))),
            dequeue_rx: Arc::new(Mutex::new(dequeue_rx)),
            properties,
        }
    }

    pub async fn take_enqueued(&self) -> Queue<Packet> {
        let mut queue = Queue::new();
        let mut enqueue_rx = self.enqueue_rx.lock().await;
        while let Ok(packet) = enqueue_rx.try_recv() {
            queue.push_back(packet);
        }
        queue
    }

    pub async fn wait_enqueued(&self) -> Option<Packet> {
        let mut enqueue_rx = self.enqueue_rx.lock().await;
        enqueue_rx.recv().await
    }

    pub async fn add_dequeue(&self, packet: Packet) {
        if let Some(tx) = self.dequeue_tx.lock().await.deref() {
            let _ = tx.send(packet);
        }
    }
}

#[async_trait]
impl PacketLayer for MockPacketLayer {
    async fn send(&self, packet: Packet) -> Result<(), Error> {
        if let Some(tx) = self.enqueue_tx.lock().await.deref() {
            let _ = tx.send(packet);
        }
        Ok(())
    }

    async fn recv(&self) -> Result<Packet, Error> {
        let mut dequeue_rx = self.dequeue_rx.lock().await;
        select! {
            biased;
            Some(packet) = dequeue_rx.recv() => Ok(packet),
            _ = tokio::time::sleep(self.properties.trans_timeout) => Err(Error::TimedOut),
            else => Err(Error::Closed)
        }
    }

    async fn close(&self) {
        self.dequeue_tx.lock().await.take();
    }

    async fn abort(&self) {
        self.dequeue_tx.lock().await.take();
    }
}
