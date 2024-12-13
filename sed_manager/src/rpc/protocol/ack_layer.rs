use async_trait::async_trait;
use std::collections::VecDeque as Queue;
use std::ops::DerefMut;
use std::sync::Arc;
use tokio::select;
use tokio::sync::{mpsc, Mutex, OnceCell};
use tokio::task::{yield_now, JoinError, JoinHandle};
use tokio::time::sleep;

use super::cache::Cache;
use super::packet_layer::PacketLayer;
use super::sequencer::{AckAction, Sequencer};
use super::with_copy::with_copy;
use crate::messaging::packet::{AckType, Packet};
use crate::rpc::error::Error;
use crate::rpc::properties::Properties;
use crate::sync::fence::Fence;

struct ReceiveTask {
    items: Mutex<mpsc::UnboundedReceiver<Result<Packet, Error>>>,
    task: Mutex<JoinHandle<()>>,
}

struct State {
    properties: Properties,
    next_layer: Box<dyn PacketLayer>,
    acknowledged: Fence,         // Sequence number that have been acknowledged by the remote.
    cache: Mutex<Cache>,         // Caches the packages that have been sent until they are acknowledged by remote.
    sequencer: Mutex<Sequencer>, // Tracks the sequence number of the received packets.
    recv_task: OnceCell<ReceiveTask>,
}

pub struct AcknowledgementLayer {
    state: Arc<State>,
}

impl AcknowledgementLayer {
    pub fn new(next_layer: Box<dyn PacketLayer>, properties: Properties) -> Self {
        let state = State {
            next_layer: next_layer,
            properties,
            acknowledged: Fence::new(),
            cache: Mutex::new(Cache::new()),
            sequencer: Mutex::new(Sequencer::new()),
            recv_task: OnceCell::new(),
        };
        Self { state: Arc::new(state) }
    }
}

#[async_trait]
impl PacketLayer for AcknowledgementLayer {
    async fn send(&self, packet: Packet) -> Result<(), Error> {
        self.state.clone().send(packet).await
    }

    async fn recv(&self) -> Result<Packet, Error> {
        self.state.clone().recv().await
    }

    async fn close(&self) {
        self.state.close().await
    }

    async fn abort(&self) {
        self.state.abort().await
    }
}

impl State {
    async fn get_or_init_recv_task(self: &Arc<Self>) -> &ReceiveTask {
        let self_clone = self.clone();
        let init_func = move || async move {
            let (tx, rx) = mpsc::unbounded_channel();
            let recv_task = tokio::spawn(async move { self_clone.recv_task(tx).await });
            ReceiveTask { items: Mutex::new(rx), task: Mutex::new(recv_task) }
        };
        self.recv_task.get_or_init(init_func).await
    }

    async fn enqueue_packet(&self, packet: Packet) -> Result<Option<u32>, Error> {
        let mut cache = self.cache.lock().await;
        let packet = self.steal_ack_nak(packet).await;
        if !packet.is_empty() {
            Ok(Some(cache.enqueue(packet)))
        } else {
            Ok(None)
        }
    }

    async fn flush(&self, cache: &mut Cache) -> Result<(), Error> {
        while let Some(packet) = cache.next() {
            self.next_layer.send(packet).await?;
        }
        Ok(())
    }

    async fn resend_timeout(&self, sequence_number: u32) -> Result<(), Error> {
        let mut cache = self.cache.lock().await;
        if cache.front_sequence_number() == sequence_number {
            cache.rewind();
            self.flush(&mut cache).await
        } else {
            Ok(())
        }
    }

    async fn confirm_packet(&self, sequence_number: u32) -> Result<(), Error> {
        let num_attempts = std::cmp::max(1, self.properties.max_retries) - 1;
        for _ in 0..num_attempts {
            select! {
                biased;
                Ok(_) = self.acknowledged.wait(sequence_number as u64) => return Ok(()),
                _ = sleep(self.properties.timeout) => self.resend_timeout(sequence_number).await?,
                else => return Err(Error::AbortedByHost),
            };
        }
        Err(Error::TimedOut)
    }

    async fn send(self: Arc<Self>, packet: Packet) -> Result<(), Error> {
        let _ = self.get_or_init_recv_task().await;
        if let Some(sequence_number) = self.enqueue_packet(packet).await? {
            yield_now().await;
            self.flush(self.cache.lock().await.deref_mut()).await?;
            self.confirm_packet(sequence_number).await
        } else {
            Ok(())
        }
    }

    async fn send_ack_nak(&self) -> Result<(), Error> {
        // The ACK/NAK is content is added automatically by `enqueue_packet`, if there is any.
        // Hence the empty packet being sent here.
        let packet = Packet { ..Default::default() };
        if let Some(_sequence_number) = self.enqueue_packet(packet).await? {
            self.flush(self.cache.lock().await.deref_mut()).await
        } else {
            Ok(())
        }
    }

    async fn resend_ack(&self) -> Result<(), Error> {
        let cache = self.cache.lock().await;
        if let Some(back) = cache.back() {
            if !back.has_payload() && back.ack_type == AckType::ACK {
                self.next_layer.send(back.clone()).await?;
            }
        }
        Ok(())
    }

    async fn recv(self: Arc<Self>) -> Result<Packet, Error> {
        let recv_task = self.get_or_init_recv_task().await;
        let mut rx = recv_task.items.lock().await;
        if let Some(result) = rx.recv().await {
            result
        } else {
            Err(Error::Closed)
        }
    }

    async fn recv_task(self: Arc<Self>, recv_queue: mpsc::UnboundedSender<Result<Packet, Error>>) {
        let mut tasks = Queue::new();
        loop {
            let task_result = join_tasks(take_finished_tasks(&mut tasks).into_iter()).await;
            let packet_result = self.clone().recv_one(&mut tasks).await;
            let error = task_result.is_err() || packet_result.is_err();
            recv_queue.send(packet_result).unwrap();
            if let Err(err) = task_result {
                recv_queue.send(Err(err)).unwrap();
            }
            if error {
                break;
            }
        }
        let _ = join_tasks(tasks.into_iter()).await;
        self.next_layer.close().await;
    }

    async fn recv_one(self: Arc<Self>, tasks: &mut Queue<JoinHandle<Result<(), Error>>>) -> Result<Packet, Error> {
        loop {
            let packet = self.next_layer.recv().await?;
            let (packet, action) = self.control_sequence_number(packet).await;
            let task = tokio::spawn(with_copy!(self, self_, self_.react_sequence_number(action)));
            tasks.push_back(task);
            if let Some(packet) = packet {
                break self.handle_ack_nak(packet).await;
            };
        }
    }

    async fn steal_ack_nak(&self, packet: Packet) -> Packet {
        let mut sequencer = self.sequencer.lock().await;
        let (ack_type, sn) = sequencer.take().unwrap_or((AckType::None, 0));
        Packet { ack_type: ack_type, acknowledgement: sn, ..packet }
    }

    async fn control_sequence_number(&self, packet: Packet) -> (Option<Packet>, AckAction) {
        let mut sequencer = self.sequencer.lock().await;
        let action = sequencer.update(packet.sequence_number);
        let payload_action = if packet.has_payload() { AckAction::ACK } else { AckAction::Pass };
        match action {
            AckAction::ACK => (Some(packet), payload_action),
            AckAction::Pass => (Some(packet), AckAction::Pass),
            _ => (None, action),
        }
    }

    async fn react_sequence_number(self: Arc<Self>, action: AckAction) -> Result<(), Error> {
        if action == AckAction::ACK {
            tokio::time::sleep(self.properties.timeout / 4).await;
        }
        match action {
            AckAction::ACK => self.send_ack_nak().await,
            AckAction::NAK => self.send_ack_nak().await,
            AckAction::Resend => self.resend_ack().await,
            AckAction::Ignore => Ok(()),
            AckAction::Pass => Ok(()),
        }
    }

    async fn handle_ack_nak(&self, packet: Packet) -> Result<Packet, Error> {
        let ack_type = packet.ack_type;
        let sn = packet.acknowledgement;
        match ack_type {
            AckType::ACK => {
                self.acknowledged.signal(sn as u64);
                let mut cache = self.cache.lock().await;
                cache.ack(sn);
            }
            AckType::NAK => {
                let acked_sn = std::cmp::max(1, sn) - 1;
                self.acknowledged.signal(acked_sn as u64);
                let mut cache = self.cache.lock().await;
                cache.ack(acked_sn);
                cache.rewind();
                self.flush(&mut cache).await?;
            }
            AckType::None => (),
        };
        Ok(packet)
    }

    async fn close(&self) {
        if let Some(recv_task) = self.recv_task.get() {
            let mut recv_task = recv_task.task.lock().await;
            let _ = recv_task.deref_mut().await;
        };
        self.next_layer.close().await;
    }

    async fn abort(&self) {
        self.next_layer.abort().await;
        self.acknowledged.close();

        if let Some(recv_task) = self.recv_task.get() {
            let mut recv_task = recv_task.task.lock().await;
            let _ = recv_task.deref_mut().await;
        };
    }
}

fn collapse_join_result(result: Result<Result<(), Error>, JoinError>) -> Result<(), Error> {
    match result {
        Ok(inner) => inner,
        Err(join_error) => {
            if join_error.is_cancelled() {
                panic!("task aborted unexpectedly");
            } else if join_error.is_panic() {
                std::panic::resume_unwind(join_error.into_panic());
            } else {
                panic!("unknown join error: {}", join_error);
            }
        }
    }
}

fn merge_results(a: Result<(), Error>, b: Result<(), Error>) -> Result<(), Error> {
    if a.is_err() {
        a
    } else {
        b
    }
}

fn take_finished_tasks(tasks: &mut Queue<JoinHandle<Result<(), Error>>>) -> Vec<JoinHandle<Result<(), Error>>> {
    let mut finished_tasks = Vec::new();
    while tasks.front().is_some_and(|task| task.is_finished()) {
        finished_tasks.push(tasks.pop_front().unwrap());
    }
    finished_tasks
}

async fn join_tasks(tasks: impl Iterator<Item = JoinHandle<Result<(), Error>>>) -> Result<(), Error> {
    let mut result = Ok(());
    for task in tasks {
        result = merge_results(result, collapse_join_result(task.await));
    }
    result
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::messaging::packet::{SubPacket, SubPacketKind};
    use crate::rpc::protocol::packet_layer::MockPacketLayer;

    use super::*;

    async fn join<Output>(handle: tokio::task::JoinHandle<Output>) -> Output
    where
        Output: 'static,
    {
        match handle.await {
            Ok(result) => result,
            Err(err) => {
                if err.is_cancelled() {
                    panic!("task was not supposed to be cancelled");
                } else {
                    std::panic::resume_unwind(err.into_panic());
                }
            }
        }
    }

    fn make_data(data: u8) -> Packet {
        let sub_packet = SubPacket { kind: SubPacketKind::Data, payload: vec![data].into() };
        Packet { payload: vec![sub_packet].into(), ..Default::default() }
    }

    fn make_cc(credit: u32) -> Packet {
        let sub_packet = SubPacket { kind: SubPacketKind::Data, payload: Vec::from(credit.to_be_bytes()).into() };
        Packet { payload: vec![sub_packet].into(), ..Default::default() }
    }

    fn make_ack(sn: u32) -> Packet {
        Packet { ack_type: AckType::ACK, acknowledgement: sn, ..Default::default() }
    }

    fn make_nak(sn: u32) -> Packet {
        Packet { ack_type: AckType::NAK, acknowledgement: sn, ..Default::default() }
    }

    fn set_sn(packet: Packet, sn: u32) -> Packet {
        Packet { sequence_number: sn, ..packet }
    }

    const PROPERTIES_ASYNC: Properties = Properties {
        max_methods: 1,
        max_subpackets: 1,
        max_packets: 1,
        max_gross_packet_size: 1004,
        max_gross_compacket_size: 1024,
        seq_numbers: true,
        ack_nak: true,
        asynchronous: true,
        buffer_mgmt: true,
        max_retries: 3,
        timeout: Duration::from_secs(15),
    };

    const PROPERTIES_ASYNC_SHORT: Properties = Properties { timeout: Duration::from_millis(1), ..PROPERTIES_ASYNC };

    #[tokio::test]
    async fn ack_data() {
        let next_layer = Box::new(MockPacketLayer::new());
        next_layer.add_dequeue(set_sn(make_data(1), 1)).await;

        let ack_layer = AcknowledgementLayer::new(next_layer.clone(), PROPERTIES_ASYNC_SHORT);

        let recv_result = ack_layer.recv().await;
        ack_layer.close().await;
        let enqueued = next_layer.take_enqueued().await;

        assert_eq!(recv_result.unwrap().sequence_number, 1);
        assert_eq!(enqueued.len(), 1);
        assert_eq!(enqueued[0].sequence_number, 1);
        assert_eq!(enqueued[0].ack_type, AckType::ACK);
        assert_eq!(enqueued[0].acknowledgement, 1);
    }

    #[tokio::test]
    async fn ack_credit() {
        let next_layer = Box::new(MockPacketLayer::new());
        next_layer.add_dequeue(set_sn(make_cc(1), 1)).await;

        let ack_layer = AcknowledgementLayer::new(next_layer.clone(), PROPERTIES_ASYNC_SHORT);

        let recv_result = ack_layer.recv().await;
        ack_layer.close().await;
        let enqueued = next_layer.take_enqueued().await;

        assert_eq!(recv_result.unwrap().sequence_number, 1);
        assert_eq!(enqueued.len(), 1);
        assert_eq!(enqueued[0].sequence_number, 1);
        assert_eq!(enqueued[0].ack_type, AckType::ACK);
        assert_eq!(enqueued[0].acknowledgement, 1);
    }

    #[tokio::test]
    async fn ack_pure_ack() {
        let next_layer = Box::new(MockPacketLayer::new());
        next_layer.add_dequeue(set_sn(make_ack(1), 1)).await;

        let ack_layer = AcknowledgementLayer::new(next_layer.clone(), PROPERTIES_ASYNC);

        let recv_result = ack_layer.recv().await;
        ack_layer.close().await;
        let enqueued = next_layer.take_enqueued().await;

        assert_eq!(recv_result.unwrap().sequence_number, 1);
        assert_eq!(enqueued.len(), 0);
    }

    #[tokio::test]
    async fn nak_missing() {
        let next_layer = Box::new(MockPacketLayer::new());
        next_layer.add_dequeue(set_sn(make_data(233), 1)).await;
        next_layer.add_dequeue(set_sn(make_data(36), 3)).await;

        let ack_layer = AcknowledgementLayer::new(next_layer.clone(), PROPERTIES_ASYNC_SHORT);

        let _ = ack_layer.recv().await;
        ack_layer.close().await;
        let enqueued = next_layer.take_enqueued().await;

        let nak = enqueued.back().unwrap();
        assert_eq!(nak.sequence_number, 1);
        assert_eq!(nak.ack_type, AckType::NAK);
        assert_eq!(nak.acknowledgement, 2);
    }

    #[tokio::test]
    async fn reack_resent() {
        let next_layer = Box::new(MockPacketLayer::new());
        let ack_layer = AcknowledgementLayer::new(next_layer.clone(), PROPERTIES_ASYNC_SHORT);
        let mut enqueued = Vec::new();

        next_layer.add_dequeue(set_sn(make_data(233), 1)).await;
        let _ = ack_layer.recv().await;
        enqueued.push(next_layer.wait_enqueued().await.unwrap());
        next_layer.add_dequeue(set_sn(make_data(36), 2)).await;
        enqueued.push(next_layer.wait_enqueued().await.unwrap());
        next_layer.add_dequeue(set_sn(make_data(233), 1)).await;
        next_layer.add_dequeue(set_sn(make_data(36), 2)).await;
        enqueued.push(next_layer.wait_enqueued().await.unwrap());
        ack_layer.close().await;

        assert_eq!(enqueued.len(), 3);
        assert_eq!(enqueued[0].sequence_number, 1);
        assert_eq!(enqueued[0].ack_type, AckType::ACK);
        assert_eq!(enqueued[0].acknowledgement, 1);
        assert_eq!(enqueued[1].sequence_number, 2);
        assert_eq!(enqueued[1].ack_type, AckType::ACK);
        assert_eq!(enqueued[1].acknowledgement, 2);
        assert_eq!(enqueued[1], enqueued[2]);
    }

    #[tokio::test]
    async fn ack_stealing() {
        let next_layer = Box::new(MockPacketLayer::new());
        next_layer.add_dequeue(set_sn(make_ack(1), 1)).await;

        let ack_layer = AcknowledgementLayer::new(next_layer.clone(), PROPERTIES_ASYNC);

        let recv_result = ack_layer.recv().await;
        let send_result = ack_layer.send(make_data(12)).await;
        ack_layer.close().await;
        let enqueued = next_layer.take_enqueued().await;

        assert_eq!(send_result, Ok(()));
        assert_eq!(recv_result.unwrap().sequence_number, 1);
        assert_eq!(enqueued.len(), 1);
        assert_eq!(enqueued[0].sequence_number, 1);
        assert_eq!(enqueued[0].payload.len(), 1);
        assert_eq!(enqueued[0].ack_type, AckType::ACK);
        assert_eq!(enqueued[0].acknowledgement, 1);
    }

    #[tokio::test]
    async fn resend_on_timeout() {
        let next_layer = Box::new(MockPacketLayer::new());
        let ack_layer = AcknowledgementLayer::new(next_layer.clone(), PROPERTIES_ASYNC_SHORT);

        let result = ack_layer.send(make_data(9)).await;
        ack_layer.close().await;
        let enqueued = next_layer.take_enqueued().await;

        assert_eq!(result, Err(Error::TimedOut));
        assert_eq!(enqueued.len(), 3);
        assert_eq!(enqueued[0].sequence_number, 1);
        assert_eq!(enqueued[1].sequence_number, 1);
        assert_eq!(enqueued[2].sequence_number, 1);
    }

    #[tokio::test]
    async fn on_acknowledgement() {
        let next_layer = Box::new(MockPacketLayer::new());
        let ack_layer = Arc::new(AcknowledgementLayer::new(next_layer.clone(), PROPERTIES_ASYNC));

        let result_task = tokio::spawn(with_copy!(ack_layer, ack_layer.send(make_data(9))));
        let enqueued = next_layer.wait_enqueued().await.unwrap();
        next_layer.add_dequeue(set_sn(make_ack(1), 1)).await;
        let result = join(result_task).await;
        ack_layer.close().await;

        assert_eq!(result, Ok(()));
        assert_eq!(enqueued.sequence_number, 1);
    }

    #[tokio::test]
    async fn resend_on_nak() {
        let next_layer = Box::new(MockPacketLayer::new());
        let ack_layer = Arc::new(AcknowledgementLayer::new(next_layer.clone(), PROPERTIES_ASYNC));
        let mut sent = Queue::new();

        let result_task_1 = tokio::spawn(with_copy!(ack_layer, ack_layer.send(make_data(9))));
        let result_task_2 = tokio::spawn(with_copy!(ack_layer, ack_layer.send(make_data(10))));
        let result_task_3 = tokio::spawn(with_copy!(ack_layer, ack_layer.send(make_data(11))));
        {
            sent.push_back(next_layer.wait_enqueued().await.unwrap());
            sent.push_back(next_layer.wait_enqueued().await.unwrap());
            sent.push_back(next_layer.wait_enqueued().await.unwrap());
        }
        next_layer.add_dequeue(set_sn(make_nak(2), 1)).await;
        {
            sent.push_back(next_layer.wait_enqueued().await.unwrap());
            sent.push_back(next_layer.wait_enqueued().await.unwrap());
        }
        next_layer.add_dequeue(set_sn(make_ack(3), 2)).await;
        let result_1 = join(result_task_1).await;
        let result_2 = join(result_task_2).await;
        let result_3 = join(result_task_3).await;
        ack_layer.close().await;

        assert_eq!(result_1, Ok(()));
        assert_eq!(result_2, Ok(()));
        assert_eq!(result_3, Ok(()));
        let sent_sns: Vec<_> = sent.iter().map(|p| p.sequence_number).collect();
        assert_eq!(sent_sns, vec![1, 2, 3, 2, 3]);
    }
}
