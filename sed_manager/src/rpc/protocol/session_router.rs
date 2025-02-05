use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};

use crate::messaging::packet::Packet;
use crate::rpc::error::Error;

use super::session_endpoint::SessionEndpoint;
use super::traits::PacketLayer;

#[derive(Debug, Hash, PartialEq, Eq)]
struct Key {
    hsn: u32,
    tsn: u32,
}

struct Channel {
    tx: mpsc::UnboundedSender<Packet>,
    rx: Mutex<mpsc::UnboundedReceiver<Packet>>,
}

pub struct SessionRouter {
    routing_table: RwLock<HashMap<Key, Channel>>,
    next_layer: Box<dyn PacketLayer>,
}

impl SessionRouter {
    pub fn new(next_layer: Box<dyn PacketLayer>) -> Self {
        Self { routing_table: HashMap::new().into(), next_layer }
    }

    pub async fn open(self: Arc<Self>, host_session_number: u32, tper_session_number: u32) -> Option<SessionEndpoint> {
        let key = Key::new(host_session_number, tper_session_number);
        let mut routing_table = self.routing_table.write().await;
        if routing_table.contains_key(&key) {
            None
        } else {
            assert!(routing_table.insert(key, Channel::new()).is_none());
            Some(SessionEndpoint::new(host_session_number, tper_session_number, self.clone()))
        }
    }

    pub async fn close(&self, host_session_number: u32, tper_session_number: u32) {
        let key = Key::new(host_session_number, tper_session_number);
        self.routing_table.write().await.remove(&key);
    }

    pub async fn send(&self, packet: Packet) -> Result<(), Error> {
        self.next_layer.send(packet).await
    }

    pub async fn recv(&self, host_session_number: u32, tper_session_number: u32) -> Result<Packet, Error> {
        loop {
            if let Some(packet) = self.peek(host_session_number, tper_session_number).await? {
                break Ok(packet);
            }
            if let Some(packet) = self.pull(host_session_number, tper_session_number).await? {
                break Ok(packet);
            }
        }
    }

    pub async fn peek(&self, host_session_number: u32, tper_session_number: u32) -> Result<Option<Packet>, Error> {
        let key = Key::new(host_session_number, tper_session_number);
        let routing_table = self.routing_table.read().await;
        if let Some(channel) = routing_table.get(&key) {
            let mut rx = channel.rx.lock().await;
            if let Ok(packet) = rx.try_recv() {
                Ok(Some(packet))
            } else {
                Ok(None)
            }
        } else {
            Err(Error::Closed)
        }
    }

    pub async fn pull(&self, host_session_number: u32, tper_session_number: u32) -> Result<Option<Packet>, Error> {
        let key = Key::new(host_session_number, tper_session_number);
        let mut routing_table = self.routing_table.write().await;
        if let Some(channel) = routing_table.get_mut(&key) {
            let rx = channel.rx.get_mut();
            if let Ok(packet) = rx.try_recv() {
                Ok(Some(packet))
            } else {
                let packet = self.next_layer.recv().await?;
                let key = Key::new(packet.host_session_number, packet.tper_session_number);
                if let Some(channel) = routing_table.get_mut(&key) {
                    let _ = channel.tx.send(packet);
                }
                Ok(None)
            }
        } else {
            Err(Error::Closed)
        }
    }
}

impl Key {
    fn new(hsn: u32, tsn: u32) -> Self {
        Self { hsn, tsn }
    }
}

impl Channel {
    fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self { tx, rx: rx.into() }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::rpc::Properties;
    use crate::with_copy::with_copy;

    use super::super::test::MockPacketLayer;
    use super::*;

    const PROPERTIES_SHORT: Properties = Properties { trans_timeout: Duration::from_millis(10), ..Properties::ASSUMED };

    fn make_session(host_session_number: u32, tper_session_number: u32) -> Packet {
        Packet { host_session_number, tper_session_number, ..Default::default() }
    }

    #[tokio::test]
    async fn send_multiple_sessions() {
        let next_layer = Box::new(MockPacketLayer::new(Properties::ASSUMED));
        let router = Arc::new(SessionRouter::new(next_layer.clone()));

        let s1 = router.clone().open(100, 101).await.unwrap();
        let s2 = router.clone().open(200, 201).await.unwrap();

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
    async fn recv_multiple_sessions() {
        let next_layer = Box::new(MockPacketLayer::new(Properties::ASSUMED));
        let router = Arc::new(SessionRouter::new(next_layer.clone()));

        let s1 = Arc::new(router.clone().open(100, 101).await.unwrap());
        let s2 = Arc::new(router.clone().open(200, 201).await.unwrap());

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

    #[tokio::test]
    async fn recv_timeout_other_active() {
        let next_layer = Box::new(MockPacketLayer::new(PROPERTIES_SHORT));
        let router = Arc::new(SessionRouter::new(next_layer.clone()));

        let s1 = Arc::new(router.clone().open(100, 101).await.unwrap());
        let s2 = Arc::new(router.clone().open(200, 201).await.unwrap());

        next_layer.add_dequeue(make_session(100, 101)).await;
        assert!(s2.recv().await.is_err_and(|err| err == Error::TimedOut));
        assert!(s1.recv().await.is_ok());

        s1.close().await;
        s2.close().await;
    }

    #[tokio::test]
    async fn recv_timeout_inactive() {
        let next_layer = Box::new(MockPacketLayer::new(PROPERTIES_SHORT));
        let router = Arc::new(SessionRouter::new(next_layer.clone()));

        let s2 = Arc::new(router.clone().open(200, 201).await.unwrap());
        assert!(s2.recv().await.is_err_and(|err| err == Error::TimedOut));

        s2.close().await;
    }
}
