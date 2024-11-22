use crate::messaging::packet::{
    AckType, Packet, SubPacket, SubPacketKind, CREDIT_CONTROL_SUB_PACKET_LEN, PACKET_HEADER_LEN, SUB_PACKET_HEADER_LEN,
};
use crate::rpc::error::Error;
use crate::rpc::pipeline::{
    BufferedSend, LinearProcess, Process, PullInput, PullOutput, PushInput, PushOutput, Receive, UnbufferedSend,
};
use crate::serialization::with_len::WithLen;

use super::super::properties::Properties;

use tokio::select;

pub struct Broadcast<T: Sync + Send + Clone> {
    pub input: PushInput<T>,
    pub outputs: Vec<PushOutput<T>>,
}

impl<T: Sync + Send + Clone> Broadcast<T> {
    pub fn new(num_outputs: usize) -> Self {
        let mut outputs = Vec::new();
        for _ in 0..num_outputs {
            outputs.push(PushOutput::<T>::new());
        }
        Self { input: PushInput::new(), outputs: outputs }
    }
}

impl<T: Sync + Send + Clone> Process for Broadcast<T> {
    type Output = ();
    type Error = ();
    async fn update(&mut self) -> Result<Option<Self::Output>, Self::Error> {
        if let Some(item) = self.input.recv().await {
            for output in &mut self.outputs {
                let _ = output.send(item.clone());
            }
            Ok(None)
        } else {
            Ok(Some(()))
        }
    }
}

pub struct PullBridge<Item: Sync + Send> {
    pub input: PullInput<Item>,
    pub output: PullOutput<Item>,
    pub cancel: tokio_util::sync::CancellationToken,
}

impl<Item: Sync + Send> Process for PullBridge<Item> {
    type Output = ();
    type Error = ();
    async fn update(&mut self) -> Result<Option<Self::Output>, Self::Error> {
        select! {
            biased;
            Some(item) = self.input.recv() => {
                let _ = self.output.send(item).await;
                Ok(None)
            }
            _ = self.cancel.cancelled() => Ok(Some(())),
            else => Ok(Some(()))
        }
    }
}

impl<Item: Sync + Send> LinearProcess for PullBridge<Item> {
    type Input = PullInput<Item>;
    type Output = PullOutput<Item>;
    fn input_mut(&mut self) -> &mut Self::Input {
        &mut self.input
    }
    fn output_mut(&mut self) -> &mut Self::Output {
        &mut self.output
    }
}

pub struct PushBridge<Item: Sync + Send> {
    pub input: PushInput<Item>,
    pub output: PushOutput<Item>,
    pub cancel: tokio_util::sync::CancellationToken,
}

impl<Item: Sync + Send> Process for PushBridge<Item> {
    type Output = ();
    type Error = ();
    async fn update(&mut self) -> Result<Option<Self::Output>, Self::Error> {
        select! {
            biased;
            Some(item) = self.input.recv() => {
                let _ = self.output.send(item);
                Ok(None)
            }
            _ = self.cancel.cancelled() => Ok(Some(())),
            else => Ok(Some(()))
        }
    }
}

impl<Item: Sync + Send> LinearProcess for PushBridge<Item> {
    type Input = PushInput<Item>;
    type Output = PushOutput<Item>;
    fn input_mut(&mut self) -> &mut Self::Input {
        &mut self.input
    }
    fn output_mut(&mut self) -> &mut Self::Output {
        &mut self.output
    }
}

pub fn into_rpc_result(result: Result<(), ()>) -> Result<(), Error> {
    match result {
        Ok(value) => Ok(value),
        Err(_) => Err(Error::Unknown),
    }
}

pub async fn redirect_result<F: std::future::Future>(result: F, sender: tokio::sync::mpsc::UnboundedSender<F::Output>) {
    let _ = sender.send(result.await);
}

pub async fn aggregate_results<E>(
    cancel: tokio_util::sync::CancellationToken,
    mut results: tokio::sync::mpsc::UnboundedReceiver<Result<(), E>>,
) -> Result<(), E> {
    let mut aggregated = Ok(());
    while let Some(result) = results.recv().await {
        if !cancel.is_cancelled() {
            cancel.cancel();
        };
        if let Err(err) = result {
            if !aggregated.is_err() {
                aggregated = Err(err)
            };
        };
    }
    aggregated
}

pub fn clamp<T: Ord>(x: T, a: T, b: T) -> T {
    std::cmp::max(a, std::cmp::min(b, x))
}

pub fn round_up(x: usize, r: usize) -> usize {
    (x + r - 1) / r * r
}

pub fn get_ack_nak(packet: &Packet) -> (AckType, u32) {
    (packet.ack_type, packet.acknowledgement)
}

pub fn get_credit_control(packet: &Packet) -> u32 {
    let mut credits = 0;
    for sub_packet in packet.payload.as_slice() {
        if sub_packet.kind == SubPacketKind::CreditControl {
            let mut be_bytes = [0u8; 4];
            for (dst, src) in std::iter::zip(be_bytes.iter_mut().rev(), sub_packet.payload.iter().rev()) {
                *dst = *src;
            }
            credits += u32::from_be_bytes(be_bytes);
        }
    }
    credits
}

pub fn get_credit_value(packet: &Packet) -> u32 {
    packet
        .payload
        .iter()
        .map(|sp| -> u32 { sp.payload.len() as u32 })
        .fold(0_u32, |x, y| -> u32 { x + y })
}

pub fn get_gross_packet_len(packet: &Packet) -> usize {
    let header = PACKET_HEADER_LEN;
    let sp_headers = packet.payload.len() * SUB_PACKET_HEADER_LEN;
    let sp_payloads = packet
        .payload
        .iter()
        .map(|sp| -> usize { round_up(sp.payload.len(), 4) })
        .fold(0_usize, |x, y| -> usize { x + y });
    return header + sp_headers + sp_payloads;
}

pub fn update_ack_nak(packet: Packet, ack_type: AckType, acknowledgement: u32) -> Packet {
    Packet {
        host_session_number: packet.host_session_number,
        tper_session_number: packet.tper_session_number,
        sequence_number: packet.sequence_number,
        ack_type: ack_type,
        acknowledgement: acknowledgement,
        payload: packet.payload,
    }
}

pub fn update_credit_control(packet: Packet, credit: u32, limit: &Properties) -> (Packet, Option<Packet>) {
    let cc_sp = SubPacket { kind: SubPacketKind::CreditControl, payload: WithLen::new(credit.to_be_bytes().into()) };
    if packet.payload.len() + 1 <= limit.max_subpackets
        && get_gross_packet_len(&packet) + CREDIT_CONTROL_SUB_PACKET_LEN <= limit.max_gross_packet_size
    {
        let mut packet = packet;
        packet.payload.push(cc_sp);
        (packet, None)
    } else {
        let cc_p = Packet {
            host_session_number: 0,
            tper_session_number: 0,
            sequence_number: 0,
            ack_type: AckType::None,
            acknowledgement: 0,
            payload: WithLen::new(vec![cc_sp]),
        };
        (packet, Some(cc_p))
    }
}

pub fn update_sequence_number(packet: Packet, sequence_number: u32) -> Packet {
    Packet {
        host_session_number: packet.host_session_number,
        tper_session_number: packet.tper_session_number,
        sequence_number: sequence_number,
        ack_type: packet.ack_type,
        acknowledgement: packet.acknowledgement,
        payload: packet.payload,
    }
}

pub fn make_empty_packet() -> Packet {
    Packet {
        host_session_number: 0,
        tper_session_number: 0,
        sequence_number: 0,
        ack_type: AckType::None,
        acknowledgement: 0,
        payload: WithLen::new(vec![]),
    }
}
