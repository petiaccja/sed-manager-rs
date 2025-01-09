use std::collections::HashMap;
use std::sync::Arc;
use tokio::select;
use tokio::sync::{mpsc, Mutex};

use crate::messaging::packet::Packet;
use crate::rpc::error::Error;

use super::session_endpoint::SessionEndpoint;
use super::traits::PacketLayer;

pub struct SessionRouter {
    routing_table: Mutex<HashMap<(u32, u32), mpsc::UnboundedSender<Packet>>>,
    recv_table: Mutex<HashMap<(u32, u32), mpsc::UnboundedReceiver<Packet>>>,
    next_layer: Box<dyn PacketLayer>,
}

impl SessionRouter {
    pub fn new(next_layer: Box<dyn PacketLayer>) -> Self {
        Self { routing_table: HashMap::new().into(), recv_table: HashMap::new().into(), next_layer }
    }

    pub async fn open(self: Arc<Self>, host_session_number: u32, tper_session_number: u32) -> Option<SessionEndpoint> {
        let key = make_key(host_session_number, tper_session_number);
        let mut routing_table = self.routing_table.lock().await;
        if routing_table.contains_key(&key) {
            None
        } else {
            let (tx, rx) = mpsc::unbounded_channel();
            assert!(routing_table.insert(key, tx).is_none());
            drop(routing_table); // Release the mutex.
            self.release_receiver(host_session_number, tper_session_number, rx).await;
            Some(SessionEndpoint::new(host_session_number, tper_session_number, self.clone()))
        }
    }

    pub async fn close(&self, host_session_number: u32, tper_session_number: u32) {
        let key = make_key(host_session_number, tper_session_number);
        self.routing_table.lock().await.remove(&key);
        drop(self.borrow_receiver(host_session_number, tper_session_number));
    }

    pub async fn send(&self, packet: Packet) -> Result<(), Error> {
        self.next_layer.send(packet).await
    }

    pub async fn recv(&self, host_session_number: u32, tper_session_number: u32) -> Result<Packet, Error> {
        async fn retry_loop(
            instance: &SessionRouter,
            receiver: &mut mpsc::UnboundedReceiver<Packet>,
        ) -> Result<Packet, Error> {
            loop {
                select! {
                    biased;
                    item = receiver.recv() => break match item {
                        Some(packet) => Ok(packet),
                        None => Err(Error::Closed),
                    },
                    routing_table = instance.routing_table.lock() => {
                        // Check the receiver just in case someone put a packet in it while we were acquiring the lock.
                        // I'm not sure this is even possible with select, but better safe than sorry.
                        if let Ok(packet) = receiver.try_recv() {
                            break Ok(packet);
                        }
                        let packet = instance.next_layer.recv().await?;
                        let key = make_key(packet.host_session_number, packet.tper_session_number);
                        if let Some(sender) = routing_table.get(&key) {
                            let _ = sender.send(packet);
                        }
                    },
                };
            }
        }

        // Make sure the receiver is released!!!
        if let Some(mut receiver) = self.borrow_receiver(host_session_number, tper_session_number).await {
            let result = retry_loop(self, &mut receiver).await;
            self.release_receiver(host_session_number, tper_session_number, receiver).await;
            result
        } else {
            Err(Error::Closed)
        }
    }

    async fn borrow_receiver(
        &self,
        host_session_number: u32,
        tper_session_number: u32,
    ) -> Option<mpsc::UnboundedReceiver<Packet>> {
        let key = make_key(host_session_number, tper_session_number);
        self.recv_table.lock().await.remove(&key)
    }

    async fn release_receiver(
        &self,
        host_session_number: u32,
        tper_session_number: u32,
        receiver: mpsc::UnboundedReceiver<Packet>,
    ) {
        let key = make_key(host_session_number, tper_session_number);
        let mut recv_table = self.recv_table.lock().await;
        if !receiver.is_closed() {
            recv_table.insert(key, receiver);
        }
    }
}

fn make_key(host_session_number: u32, tper_session_number: u32) -> (u32, u32) {
    (host_session_number, tper_session_number)
}

#[cfg(test)]
mod tests {
    use crate::rpc::protocol::with_copy::with_copy;

    use super::super::test::MockPacketLayer;
    use super::*;

    fn make_session(host_session_number: u32, tper_session_number: u32) -> Packet {
        Packet { host_session_number, tper_session_number, ..Default::default() }
    }

    #[tokio::test]
    async fn send_multiple_sessions() {
        let next_layer = Box::new(MockPacketLayer::new());
        let hub = Arc::new(SessionRouter::new(next_layer.clone()));

        let s1 = hub.clone().open(100, 101).await.unwrap();
        let s2 = hub.clone().open(200, 201).await.unwrap();

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
        let next_layer = Box::new(MockPacketLayer::new());
        let hub = Arc::new(SessionRouter::new(next_layer.clone()));

        let s1 = Arc::new(hub.clone().open(100, 101).await.unwrap());
        let s2 = Arc::new(hub.clone().open(200, 201).await.unwrap());

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
