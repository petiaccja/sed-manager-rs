use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::select;
use tokio::sync::{mpsc, Mutex};

use crate::messaging::packet::Packet;
use crate::rpc::error::Error;

use super::packet_layer::PacketLayer;

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
struct SessionID {
    host_session_number: u32,
    tper_session_number: u32,
}

struct SharedState {
    next_layer: Box<dyn PacketLayer>,
    forwarding_map: Mutex<HashMap<SessionID, mpsc::UnboundedSender<Result<Packet, Error>>>>,
    forwarding_lock: Mutex<()>,
}

pub struct MultiplexerHub {
    shared_state: Arc<SharedState>,
}

pub struct MultiplexerSession {
    id: SessionID,
    shared_state: Arc<SharedState>,
    receiver: Mutex<mpsc::UnboundedReceiver<Result<Packet, Error>>>,
}

impl SharedState {
    fn new(next_layer: Box<dyn PacketLayer>) -> Self {
        Self { next_layer, forwarding_map: HashMap::new().into(), forwarding_lock: ().into() }
    }

    async fn add_session(&self, id: SessionID) -> Option<mpsc::UnboundedReceiver<Result<Packet, Error>>> {
        let mut forwarding_map = self.forwarding_map.lock().await;
        if !forwarding_map.contains_key(&id) {
            let (tx, rx) = mpsc::unbounded_channel();
            forwarding_map.insert(id, tx);
            Some(rx)
        } else {
            None
        }
    }

    async fn remove_session(&self, id: SessionID) {
        let mut forwarding_map = self.forwarding_map.lock().await;
        forwarding_map.remove(&id);
    }

    async fn forward_one<'a>(&self, _guard: tokio::sync::MutexGuard<'a, ()>) -> Result<(), Error> {
        let packet = self.next_layer.recv().await?;
        let id = SessionID {
            tper_session_number: packet.tper_session_number,
            host_session_number: packet.host_session_number,
        };
        let forwarding_map = self.forwarding_map.lock().await;
        if let Some(sender) = forwarding_map.get(&id) {
            let _ = sender.send(Ok(packet));
        };
        Ok(())
    }
}

impl MultiplexerHub {
    pub fn new(next_layer: Box<dyn PacketLayer>) -> Self {
        Self { shared_state: Arc::new(SharedState::new(next_layer)) }
    }

    pub async fn create_session(&self, host_session_number: u32, tper_session_number: u32) -> Option<MultiplexerSession> {
        let id = SessionID { host_session_number, tper_session_number };
        if let Some(rx) = self.shared_state.add_session(id).await {
            Some(MultiplexerSession { id, shared_state: self.shared_state.clone(), receiver: rx.into() })
        } else {
            None
        }
    }
}

#[async_trait]
impl PacketLayer for MultiplexerSession {
    async fn send(&self, packet: Packet) -> Result<(), Error> {
        let packet = Packet {
            tper_session_number: self.id.tper_session_number,
            host_session_number: self.id.host_session_number,
            ..packet
        };
        self.shared_state.next_layer.send(packet).await
    }

    async fn recv(&self) -> Result<Packet, Error> {
        let mut receiver = self.receiver.lock().await;
        loop {
            select! {
                biased;
                result = receiver.recv() => {
                    break match result {
                        Some(result) => result,
                        None => Err(Error::Closed),
                    };
                },
                guard = self.shared_state.forwarding_lock.lock() => {
                    self.shared_state.forward_one(guard).await?;
                }
            }
        }
    }

    async fn close(&self) {
        self.shared_state.remove_session(self.id).await;
    }

    async fn abort(&self) {
        self.shared_state.remove_session(self.id).await;
    }
}

#[cfg(test)]
mod tests {
    use crate::rpc::protocol::with_copy::with_copy;

    use super::super::packet_layer::MockPacketLayer;
    use super::*;

    fn make_session(host_session_number: u32, tper_session_number: u32) -> Packet {
        Packet { host_session_number, tper_session_number, ..Default::default() }
    }

    #[tokio::test]
    async fn multiplex() {
        let next_layer = Box::new(MockPacketLayer::new());
        let hub = MultiplexerHub::new(next_layer.clone());

        let s1 = hub.create_session(100, 101).await.unwrap();
        let s2 = hub.create_session(200, 201).await.unwrap();

        s1.send(Packet::default()).await.unwrap();
        s2.send(Packet::default()).await.unwrap();
        s1.send(Packet::default()).await.unwrap();
        s1.close().await;
        s2.close().await;

        let enqueued = next_layer.take_enqueued().await;
        assert_eq!(enqueued.len(), 3);
        assert_eq!(enqueued[0].host_session_number, 100);
        assert_eq!(enqueued[0].tper_session_number, 101);
        assert_eq!(enqueued[1].host_session_number, 200);
        assert_eq!(enqueued[1].tper_session_number, 201);
        assert_eq!(enqueued[2].host_session_number, 100);
        assert_eq!(enqueued[2].tper_session_number, 101);
    }

    #[tokio::test]
    async fn demultiplex() {
        let next_layer = Box::new(MockPacketLayer::new());
        let hub = MultiplexerHub::new(next_layer.clone());

        let s1 = Arc::new(hub.create_session(100, 101).await.unwrap());
        let s2 = Arc::new(hub.create_session(200, 201).await.unwrap());

        let mut received = Vec::new();

        let r1 = tokio::spawn(with_copy!(s1, s1.recv()));
        next_layer.add_dequeue(make_session(100, 101)).await;
        next_layer.add_dequeue(make_session(100, 101)).await;
        received.push(r1.await.unwrap().unwrap());

        let r2 = tokio::spawn(with_copy!(s2, s2.recv()));
        next_layer.add_dequeue(make_session(200, 201)).await;
        received.push(r2.await.unwrap().unwrap());

        let r3 = tokio::spawn(with_copy!(s1, s1.recv()));
        received.push(r3.await.unwrap().unwrap());

        s1.close().await;
        s2.close().await;

        assert_eq!(received[0].host_session_number, 100);
        assert_eq!(received[0].tper_session_number, 101);
        assert_eq!(received[1].host_session_number, 200);
        assert_eq!(received[1].tper_session_number, 201);
        assert_eq!(received[2].host_session_number, 100);
        assert_eq!(received[2].tper_session_number, 101);
    }
}
