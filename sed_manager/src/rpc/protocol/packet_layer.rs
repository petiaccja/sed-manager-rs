use super::super::error::Error;
use super::super::pipeline::{Process, PullInput, PullOutput, PushInput, PushOutput};
use super::utils::{
    aggregate_results, clamp, get_ack_nak, get_credit_control, get_credit_value, into_rpc_result, make_empty_packet,
    update_ack_nak, update_credit_control, update_sequence_number, Broadcast,
};
use crate::messaging::packet::{AckType, Packet};
use crate::rpc::pipeline::{connect, BoxedProcess, BufferedSend, LinearProcess, Receive, UnbufferedSend};
use crate::rpc::properties::Properties;
use crate::rpc::protocol::utils::{PullBridge, PushBridge};

use std::collections::VecDeque as Queue;
use std::future::Future;
use std::pin::Pin;
use tokio::select;
use tokio::sync::mpsc;

pub struct PacketLayer {
    pub input_outbound: PullInput<Packet>,
    pub input_inbound: PushInput<Packet>,
    pub credit: PushInput<u32>,
    pub output_outbound: PullOutput<Packet>,
    pub output_inbound: PushOutput<Packet>,
    properties: Properties,
    initial_credit: u32,
}

struct AddAckNak {
    pub input: PullInput<Packet>,
    pub output: PullOutput<Packet>,
    pub ack_nak: PushInput<(AckType, u32)>,
}

struct AddCreditControl {
    pub input: PullInput<Packet>,
    pub output: PullOutput<Packet>,
    pub credit: PushInput<u32>,
    properties: Properties,
}

struct Hold {
    pub input: PullInput<Packet>,
    pub output: PullOutput<Packet>,
    pub credit: PushInput<u32>,
    current_credit: u32,
}

struct AssignSequenceNumber {
    pub input: PullInput<Packet>,
    pub output: PullOutput<Packet>,
    sn_to_assign: u32,
}

struct Timeout {
    pub input: PullInput<Packet>,
    pub output: PullOutput<Packet>,
    pub ack_nak: PushInput<(AckType, u32)>,
    pub timeout: PushOutput<u32>,
    properties: Properties,
    acked: u32,
    timers: Queue<Timer>,
}

struct Cache {
    pub input: PullInput<Packet>,
    pub output: PullOutput<Packet>,
    pub ack_nak: PushInput<(AckType, u32)>,
    pub timeout: PushInput<u32>,
    packets: Queue<Packet>,
    sn_to_send: u32,
}

struct CheckSequenceNumber {
    pub input: PushInput<Packet>,
    pub output: PushOutput<Packet>,
    pub ack_nak: PushOutput<(AckType, u32)>,
    expected_sn: u32,
    nakd_sn: u32,
}

struct ExtractCreditControl {
    pub input: PushInput<Packet>,
    pub output: PushOutput<Packet>,
    credit: PushOutput<u32>,
}

struct ExtractAckNak {
    pub input: PushInput<Packet>,
    pub output: PushOutput<Packet>,
    ack_nak: PushOutput<(AckType, u32)>,
}

struct Timer {
    sequence_number: u32,
    retry: u32,
    future: Pin<Box<dyn Future<Output = ()> + Send + Sync + 'static>>,
}

trait BoxedLinearProcess<Output, Error, InputPort, OutputPort>:
    BoxedProcess<Output = Output, Error = Error> + LinearProcess<Input = InputPort, Output = OutputPort>
{
}

impl<T: BoxedProcess + LinearProcess>
    BoxedLinearProcess<
        <T as BoxedProcess>::Output,
        <T as BoxedProcess>::Error,
        <T as LinearProcess>::Input,
        <T as LinearProcess>::Output,
    > for T
{
}

struct CombinedProcess {
    outbound: Vec<Box<dyn BoxedLinearProcess<(), Error, PullInput<Packet>, PullOutput<Packet>>>>,
    inbound: Vec<Box<dyn BoxedLinearProcess<(), Error, PushInput<Packet>, PushOutput<Packet>>>>,
    aux: Vec<Box<dyn BoxedProcess<Output = (), Error = ()>>>,
    cancel: tokio_util::sync::CancellationToken,
}

impl CombinedProcess {
    fn append_outbound<T>(&mut self, mut process: T)
    where
        T: BoxedLinearProcess<(), Error, PullInput<Packet>, PullOutput<Packet>> + 'static,
    {
        if let Some(last) = self.outbound.last_mut() {
            connect(last.output_mut(), process.input_mut());
        }
        self.outbound.push(Box::new(process));
    }

    fn append_inbound<T>(&mut self, mut process: T)
    where
        T: BoxedLinearProcess<(), Error, PushInput<Packet>, PushOutput<Packet>> + 'static,
    {
        if let Some(last) = self.inbound.last_mut() {
            connect(last.output_mut(), process.input_mut());
        }
        self.inbound.push(Box::new(process));
    }

    fn append_aux<T>(&mut self, process: T)
    where
        T: BoxedProcess<Output = (), Error = ()> + 'static,
    {
        self.aux.push(Box::new(process));
    }
}

impl PacketLayer {
    pub fn new(properties: Properties, initial_credit: u32) -> Self {
        Self {
            input_outbound: PullInput::new(),
            output_outbound: PullOutput::new(),
            input_inbound: PushInput::new(),
            output_inbound: PushOutput::new(),
            credit: PushInput::new(),
            properties: properties,
            initial_credit: initial_credit,
        }
    }

    fn create_combined_pipeline(self) -> CombinedProcess {
        // IMPORTANT:
        // It's absolutely crucial to append the sub-processes in the correct order or else the pipeline
        // will be gibberish and will not work correctly.
        let mut combined_process = CombinedProcess {
            outbound: Vec::new(),
            inbound: Vec::new(),
            aux: Vec::new(),
            cancel: tokio_util::sync::CancellationToken::new(),
        };

        combined_process.append_aux(PullBridge {
            input: self.input_outbound,
            output: PullOutput::new(),
            cancel: combined_process.cancel.clone(),
        });
        combined_process.append_aux(PushBridge {
            input: self.input_inbound,
            output: PushOutput::new(),
            cancel: combined_process.cancel.clone(),
        });

        if self.properties.ack_nak {
            let mut add_ack_nak = AddAckNak::new();
            let mut check_sn = CheckSequenceNumber::new();
            connect(&mut check_sn.ack_nak, &mut add_ack_nak.ack_nak);
            combined_process.append_outbound(add_ack_nak);
            combined_process.append_inbound(check_sn);
        };

        if self.properties.buffer_mgmt {
            let mut add_credit_control = AddCreditControl::new(self.properties.clone());
            let mut hold = Hold::new(self.initial_credit);
            let mut credit_bridge =
                PushBridge { input: self.credit, output: PushOutput::new(), cancel: combined_process.cancel.clone() };
            let mut extract_credit_control = ExtractCreditControl::new();
            connect(&mut add_credit_control.credit, &mut credit_bridge.output);
            connect(&mut extract_credit_control.credit, &mut hold.credit);
            combined_process.append_outbound(add_credit_control);
            combined_process.append_outbound(hold);
            combined_process.append_inbound(extract_credit_control);
            combined_process.append_aux(credit_bridge);
        }

        if self.properties.seq_numbers | self.properties.ack_nak {
            combined_process.append_outbound(AssignSequenceNumber::new());
        }

        if self.properties.ack_nak {
            let mut timeout = Timeout::new(self.properties.clone());
            let mut cache = Cache::new();
            let mut broadcast_ack_nak = Broadcast::new(2);
            let mut extract_ack_nak = ExtractAckNak::new();
            connect(&mut broadcast_ack_nak.outputs[0], &mut timeout.ack_nak);
            connect(&mut broadcast_ack_nak.outputs[1], &mut cache.ack_nak);
            connect(&mut timeout.timeout, &mut cache.timeout);
            connect(&mut extract_ack_nak.ack_nak, &mut broadcast_ack_nak.input);
            combined_process.append_outbound(timeout);
            combined_process.append_outbound(cache);
            combined_process.append_inbound(extract_ack_nak);
            combined_process.append_aux(broadcast_ack_nak);
        }

        *combined_process.outbound.last_mut().unwrap().output_mut() = self.output_outbound;
        *combined_process.inbound.last_mut().unwrap().output_mut() = self.output_inbound;

        combined_process
    }
}

impl AddAckNak {
    pub fn new() -> Self {
        Self { input: PullInput::new(), output: PullOutput::new(), ack_nak: PushInput::new() }
    }
}

impl AddCreditControl {
    pub fn new(properties: Properties) -> Self {
        Self { input: PullInput::new(), output: PullOutput::new(), credit: PushInput::new(), properties: properties }
    }
}

impl Hold {
    pub fn new(initial_credit: u32) -> Self {
        Self {
            input: PullInput::new(),
            output: PullOutput::new(),
            credit: PushInput::new(),
            current_credit: initial_credit,
        }
    }
}

impl AssignSequenceNumber {
    pub fn new() -> Self {
        Self { input: PullInput::new(), output: PullOutput::new(), sn_to_assign: 1 }
    }
}

impl Timeout {
    pub fn new(properties: Properties) -> Self {
        Self {
            input: PullInput::new(),
            output: PullOutput::new(),
            ack_nak: PushInput::new(),
            timeout: PushOutput::new(),
            properties: properties,
            acked: 0,
            timers: Queue::new(),
        }
    }

    pub fn start_timer(&mut self, sequence_number: u32, retry: u32) -> bool {
        // Packet already acknowledged => no action & return success.
        if sequence_number <= self.acked {
            return true;
        }

        // Packet did not exceed retry limit => launch timer & return success.
        if retry <= self.properties.max_retries {
            let delay = self.properties.timeout / 2;
            let future = tokio::time::sleep(delay);
            self.timers
                .push_back(Timer { sequence_number: sequence_number, retry: retry, future: Box::pin(future) });
            return true;
        }

        self.properties.max_retries == 0
    }

    pub fn acknowledge(&mut self, ack_type: AckType, sequence_number: u32) {
        match ack_type {
            AckType::ACK => self.acked = std::cmp::max(self.acked, sequence_number),
            AckType::NAK => self.acked = std::cmp::max(self.acked, std::cmp::max(1, sequence_number) - 1),
            _ => (),
        }
    }
}

impl Cache {
    pub fn new() -> Self {
        Self {
            input: PullInput::new(),
            output: PullOutput::new(),
            ack_nak: PushInput::new(),
            timeout: PushInput::new(),
            packets: Queue::new(),
            sn_to_send: 1,
        }
    }

    fn push(&mut self, packet: Packet) {
        if let Some(back) = self.packets.back() {
            assert!(back.sequence_number + 1 == packet.sequence_number, "packet ordering violated");
        } else {
            assert!(self.sn_to_send == packet.sequence_number);
        }
        self.packets.push_back(packet);
    }

    fn pop(&mut self) -> Option<Packet> {
        if let Some(packet) = self.find_by_sn(self.sn_to_send).cloned() {
            self.sn_to_send += 1;
            Some(packet.clone())
        } else {
            None
        }
    }

    fn acknowledge(&mut self, ack_type: AckType, sequence_number: u32) {
        match ack_type {
            AckType::ACK => {
                while self.packets.front().is_some_and(|packet| packet.sequence_number <= sequence_number) {
                    self.packets.pop_front();
                }
                self.sn_to_send = clamp(sequence_number, self.sn_to_send, self.back_sn());
            }
            AckType::NAK => {
                let previous_sn = std::cmp::max(sequence_number, 1) - 1;
                self.acknowledge(AckType::ACK, previous_sn);
                if let Some(front) = self.packets.front() {
                    self.sn_to_send = clamp(sequence_number, front.sequence_number, self.back_sn());
                }
            }
            AckType::None => (),
        }
    }

    fn timeout(&mut self, sequence_number: u32) {
        if sequence_number == self.front_sn() {
            self.sn_to_send = sequence_number;
        }
    }

    fn front_sn(&self) -> u32 {
        if let Some(front) = self.packets.front() {
            front.sequence_number
        } else {
            self.sn_to_send
        }
    }

    fn back_sn(&self) -> u32 {
        if let Some(back) = self.packets.back() {
            back.sequence_number
        } else {
            self.sn_to_send
        }
    }

    fn find_by_sn(&self, sequence_number: u32) -> Option<&Packet> {
        if let Some(front) = self.packets.front() {
            let sn_front = front.sequence_number;
            if sn_front <= sequence_number && sequence_number < sn_front + self.packets.len() as u32 {
                Some(&self.packets[(sequence_number - sn_front) as usize])
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl CheckSequenceNumber {
    fn new() -> Self {
        Self {
            input: PushInput::new(),
            output: PushOutput::new(),
            ack_nak: PushOutput::new(),
            expected_sn: 1,
            nakd_sn: 0,
        }
    }
}

impl ExtractCreditControl {
    fn new() -> Self {
        Self { input: PushInput::new(), output: PushOutput::new(), credit: PushOutput::new() }
    }
}

impl ExtractAckNak {
    fn new() -> Self {
        Self { input: PushInput::new(), output: PushOutput::new(), ack_nak: PushOutput::new() }
    }
}

impl Process for PacketLayer {
    type Output = ();
    type Error = Error;
    async fn update(&mut self) -> Result<Option<Self::Output>, Self::Error> {
        unreachable!()
    }
    async fn run(self) -> Result<Self::Output, Self::Error> {
        let combined_process = self.create_combined_pipeline();
        let (result_tx, result_rx) = mpsc::unbounded_channel::<Result<(), Error>>();

        for process in combined_process.aux {
            let copy = result_tx.clone();
            tokio::spawn(async move { copy.send(into_rpc_result(process.run().await)).unwrap() });
        }
        for process in combined_process.outbound {
            let copy = result_tx.clone();
            tokio::spawn(async move { copy.send(process.run().await).unwrap() });
        }
        for process in combined_process.inbound {
            let copy = result_tx.clone();
            tokio::spawn(async move { copy.send(process.run().await).unwrap() });
        }

        drop(result_tx);
        aggregate_results(combined_process.cancel, result_rx).await
    }
}

impl Process for AddAckNak {
    type Output = ();
    type Error = Error;
    async fn update(&mut self) -> Result<Option<Self::Output>, Self::Error> {
        select! {
            biased;
            Some((ack_type, acknowledgement)) = self.ack_nak.recv() => {
                // Fetch an existing packet, if any, and add ACK/NAK to that.
                let packet = self.input.try_recv().unwrap_or(make_empty_packet());
                let packet = update_ack_nak(packet, ack_type, acknowledgement);
                let _ = self.output.send(packet).await;
                Ok(None)
            },
            Some(packet) = self.input.recv() => {
                // Send packet through as is.
                let _ = self.output.send(packet).await;
                Ok(None)
            },
            else => Ok(Some(())),
        }
    }
}

impl Process for AddCreditControl {
    type Output = ();
    type Error = Error;
    async fn update(&mut self) -> Result<Option<Self::Output>, Self::Error> {
        select! {
            biased;
            Some(credit) = self.credit.recv() => {
                // Try fetch a packet that's already got some data to reduce interface overhead.
                let packet = self.input.try_recv().unwrap_or(make_empty_packet());
                // Update credit control and send packet through.
                let (packet, cc) = update_credit_control(packet, credit, &self.properties);
                if let Some(cc) = cc {
                    let _ = self.output.send(cc).await;
                }
                let _ = self.output.send(packet).await;
                Ok(None)
            },
            Some(packet) = self.input.recv() => {
                // No credit control, just pass the packet through.
                let _ = self.output.send(packet).await;
                Ok(None)
            },
            else => Ok(Some(())),
        }
    }
}

impl Process for Hold {
    type Output = ();
    type Error = Error;
    async fn update(&mut self) -> Result<Option<Self::Output>, Self::Error> {
        select! {
            biased;
            Some(credit) = self.credit.recv() => {
                self.current_credit += credit;
                Ok(None)
            }
            Some(packet) = self.input.recv() => {
                let credit_required = get_credit_value(&packet);
                // Wait for credits until we have enough to send the packet.
                while self.current_credit < credit_required {
                    match self.credit.recv().await {
                        Some(credit) => self.current_credit += credit,
                        None => return Err(Error::RemoteAborted),
                    }
                }
                // Send the packet and subtract the credits.
                let _ = self.output.send(packet).await;
                self.current_credit -= credit_required;
                Ok(None)
            }
            else => Ok(Some(())),
        }
    }
}

impl Process for AssignSequenceNumber {
    type Output = ();
    type Error = Error;
    async fn update(&mut self) -> Result<Option<Self::Output>, Self::Error> {
        if let Some(packet) = self.input.recv().await {
            let _ = self.output.send(update_sequence_number(packet, self.sn_to_assign)).await;
            self.sn_to_assign += 1;
            Ok(None)
        } else {
            Ok(Some(()))
        }
    }
}

impl Process for Timeout {
    type Output = ();
    type Error = Error;
    async fn update(&mut self) -> Result<Option<Self::Output>, Self::Error> {
        if let Some(timer) = self.timers.front_mut() {
            select! {
                biased;
                Some((ack_type, sn)) = self.ack_nak.recv() => {
                    #[allow(dropping_references)]
                    drop(timer); // This is so that &mut self can be used below.
                    self.acknowledge(ack_type, sn);
                    Ok(None)
                }
                Some(packet) = self.input.recv() => {
                    let _ = self.start_timer(packet.sequence_number, 1);
                    let _ = self.output.send(packet).await;
                    Ok(None)
                }
                _ = timer.future.as_mut() => {
                    let timer = self.timers.pop_front().unwrap();
                    if self.start_timer(timer.sequence_number, timer.retry + 1) {
                        let _ = self.timeout.send(timer.sequence_number);
                        Ok(None)
                    }
                    else {
                        Err(Error::TimedOut)
                    }
                }
                else => Ok(Some(())),
            }
        } else {
            select! {
                biased;
                Some((ack_type, sn)) = self.ack_nak.recv() => {
                    self.acknowledge(ack_type, sn);
                    Ok(None)
                }
                Some(packet) = self.input.recv() => {
                    let _ = self.start_timer(packet.sequence_number, 1);
                    let _ = self.output.send(packet).await;
                    Ok(None)
                }
                else => Ok(Some(())),
            }
        }
    }
}

impl Process for Cache {
    type Output = ();
    type Error = Error;
    async fn update(&mut self) -> Result<Option<Self::Output>, Self::Error> {
        if let Some(packet) = self.pop() {
            let _ = self.output.send(packet).await;
            return Ok(None);
        }
        select! {
            biased;
            Some((ack_type, sequence_number)) = self.ack_nak.recv() => {
                self.acknowledge(ack_type, sequence_number);
                Ok(None)
            }
            Some(timeout) = self.timeout.recv() => {
                self.timeout(timeout);
                Ok(None)
            }
            Some(packet) = self.input.recv() => {
                self.push(packet);
                Ok(None)
            }
            else => Ok(Some(()))
        }
    }
}

impl Process for CheckSequenceNumber {
    type Output = ();
    type Error = Error;
    async fn update(&mut self) -> Result<Option<Self::Output>, Self::Error> {
        if let Some(packet) = self.input.recv().await {
            let received_sn = packet.sequence_number;
            if received_sn == self.expected_sn {
                // Packets with payload except for ACK/NAK should not be acknowledged.
                if !packet.payload.is_empty() {
                    let _ = self.ack_nak.send((AckType::ACK, received_sn));
                }
                self.expected_sn += 1;
                let _ = self.output.send(packet);
            } else if received_sn > self.expected_sn && self.nakd_sn != self.expected_sn {
                let _ = self.ack_nak.send((AckType::NAK, self.expected_sn));
                self.nakd_sn = self.expected_sn;
            }
            Ok(None)
        } else {
            Ok(Some(()))
        }
    }
}

impl Process for ExtractCreditControl {
    type Output = ();
    type Error = Error;
    async fn update(&mut self) -> Result<Option<Self::Output>, Self::Error> {
        if let Some(packet) = self.input.recv().await {
            let cc = get_credit_control(&packet);
            if cc != 0 {
                let _ = self.credit.send(cc);
            }
            let _ = self.output.send(packet);
            Ok(None)
        } else {
            Ok(Some(()))
        }
    }
}

impl Process for ExtractAckNak {
    type Output = ();
    type Error = Error;
    async fn update(&mut self) -> Result<Option<Self::Output>, Self::Error> {
        if let Some(packet) = self.input.recv().await {
            let (ack_type, sn) = get_ack_nak(&packet);
            if ack_type != AckType::None {
                let _ = self.ack_nak.send((ack_type, sn));
            }
            let _ = self.output.send(packet);
            Ok(None)
        } else {
            Ok(Some(()))
        }
    }
}

macro_rules! impl_linear_process {
    ($process:ty, $input:ty, $output:ty) => {
        impl LinearProcess for $process {
            type Input = $input;
            type Output = $output;
            fn input_mut(&mut self) -> &mut Self::Input {
                &mut self.input
            }
            fn output_mut(&mut self) -> &mut Self::Output {
                &mut self.output
            }
        }
    };
}

impl_linear_process!(AddAckNak, PullInput<Packet>, PullOutput<Packet>);
impl_linear_process!(AddCreditControl, PullInput<Packet>, PullOutput<Packet>);
impl_linear_process!(Hold, PullInput<Packet>, PullOutput<Packet>);
impl_linear_process!(AssignSequenceNumber, PullInput<Packet>, PullOutput<Packet>);
impl_linear_process!(Timeout, PullInput<Packet>, PullOutput<Packet>);
impl_linear_process!(Cache, PullInput<Packet>, PullOutput<Packet>);
impl_linear_process!(CheckSequenceNumber, PushInput<Packet>, PushOutput<Packet>);
impl_linear_process!(ExtractCreditControl, PushInput<Packet>, PushOutput<Packet>);
impl_linear_process!(ExtractAckNak, PushInput<Packet>, PushOutput<Packet>);

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::task::yield_now;

    use crate::messaging::packet::{SubPacket, SubPacketKind};
    use crate::rpc::test_utils::{try_collect_pull, try_collect_push};
    use crate::rpc::{
        pipeline::spawn,
        properties::ASSUMED_PROPERTIES,
        test_utils::{
            collect_pull, collect_push, create_sink_pull, create_sink_push, create_source_pull, create_source_push,
            run_async_test,
        },
    };
    use crate::serialization::with_len::WithLen;

    use super::*;

    #[test]
    fn add_ack_nak_with_packet() -> Result<(), Error> {
        run_async_test(async {
            let mut process = AddAckNak::new();
            let mut packet_source = create_source_pull(&mut process.input);
            let mut ack_source = create_source_push(&mut process.ack_nak);
            let sink = create_sink_pull(&mut process.output);

            let process = spawn(process);
            packet_source.send(update_sequence_number(make_empty_packet(), 200)).await.unwrap();
            ack_source.send((AckType::ACK, 35)).unwrap();
            drop(packet_source);
            drop(ack_source);

            let packets = collect_pull(sink).await;
            assert_eq!(packets[0].sequence_number, 200);
            assert_eq!(packets[0].ack_type, AckType::ACK);
            assert_eq!(packets[0].acknowledgement, 35);

            process.await
        })
    }

    #[test]
    fn add_ack_nak_without_packet() -> Result<(), Error> {
        run_async_test(async {
            let mut process = AddAckNak::new();
            let packet_source = create_source_pull(&mut process.input);
            let mut ack_source = create_source_push(&mut process.ack_nak);
            let sink = create_sink_pull(&mut process.output);

            let process = spawn(process);
            ack_source.send((AckType::ACK, 35)).unwrap();
            drop(packet_source);
            drop(ack_source);

            let packets = collect_pull(sink).await;
            assert_eq!(packets[0].sequence_number, 0);
            assert_eq!(packets[0].ack_type, AckType::ACK);
            assert_eq!(packets[0].acknowledgement, 35);

            process.await
        })
    }

    #[test]
    fn add_credit_control_with_packet() -> Result<(), Error> {
        run_async_test(async {
            let mut process = AddCreditControl::new(ASSUMED_PROPERTIES);
            let mut packet_source = create_source_pull(&mut process.input);
            let mut credits_source = create_source_push(&mut process.credit);
            let sink = create_sink_pull(&mut process.output);

            let process = spawn(process);
            packet_source.send(update_sequence_number(make_empty_packet(), 200)).await.unwrap();
            credits_source.send(0xFE).unwrap();
            drop(packet_source);
            drop(credits_source);

            let packets = collect_pull(sink).await;
            assert_eq!(packets[0].sequence_number, 200);
            assert_eq!(packets[0].payload[0].kind, SubPacketKind::CreditControl);
            assert_eq!(packets[0].payload[0].payload[3], 0xFE);

            process.await
        })
    }

    #[test]
    fn add_credit_control_without_packet() -> Result<(), Error> {
        run_async_test(async {
            let mut process = AddCreditControl::new(ASSUMED_PROPERTIES);
            let packet_source = create_source_pull(&mut process.input);
            let mut credits_source = create_source_push(&mut process.credit);
            let sink = create_sink_pull(&mut process.output);

            let process = spawn(process);
            credits_source.send(0xFE).unwrap();
            drop(packet_source);
            drop(credits_source);

            let packets = collect_pull(sink).await;
            assert_eq!(packets[0].sequence_number, 0);
            assert_eq!(packets[0].payload[0].kind, SubPacketKind::CreditControl);
            assert_eq!(packets[0].payload[0].payload[3], 0xFE);

            process.await
        })
    }

    fn credit_test_packet(size: usize) -> Packet {
        let payload = vec![SubPacket { kind: SubPacketKind::Data, payload: WithLen::new(vec![0u8; size]) }];
        Packet {
            tper_session_number: 0,
            host_session_number: 0,
            sequence_number: 0,
            ack_type: AckType::None,
            acknowledgement: 0,
            payload: WithLen::new(payload),
        }
    }

    #[test]
    fn hold_credit_first() -> Result<(), Error> {
        run_async_test(async {
            let mut process = Hold::new(0);
            let mut input_source = create_source_pull(&mut process.input);
            let output_sink = create_sink_pull(&mut process.output);
            let mut credits_source = create_source_push(&mut process.credit);

            let process = spawn(process);
            credits_source.send(0xFF).unwrap();
            input_source.send(credit_test_packet(0xFF)).await.unwrap();

            drop(input_source);
            drop(credits_source);
            let packets = collect_pull(output_sink).await;
            assert_eq!(packets[0].payload[0].payload.len(), 0xFF);

            process.await
        })
    }

    #[test]
    fn hold_packet_first() -> Result<(), Error> {
        run_async_test(async {
            let mut process = Hold::new(0);
            let mut input_source = create_source_pull(&mut process.input);
            let mut output_sink = create_sink_pull(&mut process.output);
            let mut credits_source = create_source_push(&mut process.credit);

            let process = spawn(process);
            input_source.send(credit_test_packet(0xFF)).await.unwrap();
            assert!(output_sink.try_recv().is_none());
            credits_source.send(0xFF).unwrap();

            drop(input_source);
            drop(credits_source);
            let packets = collect_pull(output_sink).await;
            assert_eq!(packets[0].payload[0].payload.len(), 0xFF);

            process.await
        })
    }

    #[test]
    fn hold_credit_use() -> Result<(), Error> {
        run_async_test(async {
            let mut process = Hold::new(0xFF);
            let mut input_source = create_source_pull(&mut process.input);
            let output_sink = create_sink_pull(&mut process.output);
            let credits_source = create_source_push(&mut process.credit);

            input_source.send(credit_test_packet(0x34)).await.unwrap();
            process.update().await.unwrap();
            assert_eq!(process.current_credit, 0xFF - 0x34);
            let process = spawn(process);

            drop(input_source);
            drop(credits_source);
            let packets = collect_pull(output_sink).await;
            assert_eq!(packets[0].payload[0].payload.len(), 0x34);

            process.await
        })
    }

    #[test]
    fn timeout_fixed_retries() {
        let result = run_async_test(async {
            let properties = Properties { timeout: Duration::from_millis(10), max_retries: 3, ..Default::default() };
            let mut process = Timeout::new(properties);
            let mut input_source = create_source_pull(&mut process.input);
            let ack_source = create_source_push(&mut process.ack_nak);
            let output_sink = create_sink_pull(&mut process.output);
            let timeout_sink = create_sink_push(&mut process.timeout);

            let process = spawn(process);
            input_source.send(update_sequence_number(make_empty_packet(), 201)).await.unwrap();
            input_source.send(update_sequence_number(make_empty_packet(), 202)).await.unwrap();
            input_source.send(update_sequence_number(make_empty_packet(), 203)).await.unwrap();

            drop(input_source);
            drop(ack_source);
            let packets = tokio::spawn(collect_pull(output_sink));
            let timeouts = tokio::spawn(collect_push(timeout_sink));
            let result = process.await;

            let packets: Vec<_> = packets.await.unwrap().into_iter().map(|p| p.sequence_number).collect();
            let timeouts = timeouts.await.unwrap();
            assert_eq!(packets, [201, 202, 203]);
            assert_eq!(timeouts, [201, 202, 203, 201, 202, 203]);

            result
        });
        assert_eq!(std::mem::discriminant(&result.unwrap_err()), std::mem::discriminant(&Error::TimedOut));
    }

    #[test]
    fn timeout_no_retries() {
        let result = run_async_test(async {
            let properties = Properties { timeout: Duration::from_millis(10), max_retries: 0, ..Default::default() };
            let mut process = Timeout::new(properties);
            let mut input_source = create_source_pull(&mut process.input);
            let ack_source = create_source_push(&mut process.ack_nak);
            let output_sink = create_sink_pull(&mut process.output);
            let timeout_sink = create_sink_push(&mut process.timeout);

            let process = spawn(process);
            input_source.send(update_sequence_number(make_empty_packet(), 201)).await.unwrap();
            input_source.send(update_sequence_number(make_empty_packet(), 202)).await.unwrap();
            input_source.send(update_sequence_number(make_empty_packet(), 203)).await.unwrap();

            drop(input_source);
            drop(ack_source);
            let packets = tokio::spawn(collect_pull(output_sink));
            let timeouts = tokio::spawn(collect_push(timeout_sink));
            let result = process.await;

            let packets: Vec<_> = packets.await.unwrap().into_iter().map(|p| p.sequence_number).collect();
            let timeouts = timeouts.await.unwrap();
            assert_eq!(packets, [201, 202, 203]);
            assert_eq!(timeouts, []);

            result
        });
        assert!(result.is_ok());
    }

    #[test]
    fn cache_nak() -> Result<(), Error> {
        run_async_test(async {
            let mut process = Cache::new();
            let mut input_source = create_source_pull(&mut process.input);
            let mut ack_source = create_source_push(&mut process.ack_nak);
            let timeout_source = create_source_push(&mut process.timeout);
            let mut output_sink = create_sink_pull(&mut process.output);

            let process = spawn(process);
            let mut packets = vec![];

            for i in 1..=5 {
                input_source.send(update_sequence_number(make_empty_packet(), i)).await.unwrap();
                yield_now().await;
                try_collect_pull(&mut packets, &mut output_sink).await;
            }
            ack_source.send((AckType::NAK, 3)).unwrap();
            yield_now().await;
            try_collect_pull(&mut packets, &mut output_sink).await;

            drop(input_source);
            drop(ack_source);
            drop(timeout_source);
            let sn: Vec<_> = packets.into_iter().map(|p| p.sequence_number).collect();
            assert_eq!(sn, [1, 2, 3, 4, 5, 3, 4, 5]);
            process.await
        })
    }

    #[test]
    fn cache_timeout() -> Result<(), Error> {
        run_async_test(async {
            let mut process = Cache::new();
            let mut input_source = create_source_pull(&mut process.input);
            let mut ack_source = create_source_push(&mut process.ack_nak);
            let mut timeout_source = create_source_push(&mut process.timeout);
            let mut output_sink = create_sink_pull(&mut process.output);

            let process = spawn(process);
            let mut packets = vec![];

            for i in 1..=5 {
                input_source.send(update_sequence_number(make_empty_packet(), i)).await.unwrap();
                yield_now().await;
                try_collect_pull(&mut packets, &mut output_sink).await;
            }
            ack_source.send((AckType::ACK, 2)).unwrap();
            yield_now().await;
            timeout_source.send(3).unwrap();
            yield_now().await;
            timeout_source.send(5).unwrap();
            yield_now().await;
            try_collect_pull(&mut packets, &mut output_sink).await;

            drop(input_source);
            drop(ack_source);
            drop(timeout_source);
            let sn: Vec<_> = packets.into_iter().map(|p| p.sequence_number).collect();
            assert_eq!(sn, [1, 2, 3, 4, 5, 3, 4, 5]);
            process.await
        })
    }

    #[test]
    fn check_sequence_number_loss() -> Result<(), Error> {
        run_async_test(async {
            let mut process = CheckSequenceNumber::new();
            let mut input_source = create_source_push(&mut process.input);
            let mut _output_sink = create_sink_push(&mut process.output);
            let mut ack_sink = create_sink_push(&mut process.ack_nak);

            let process = spawn(process);

            for i in [1, 3, 4, 2, 3] {
                input_source.send(update_sequence_number(make_empty_packet(), i)).unwrap();
            }
            yield_now().await;

            let mut acks = vec![];
            try_collect_push(&mut acks, &mut ack_sink).await;

            drop(input_source);
            assert_eq!(
                acks,
                [
                    (AckType::ACK, 1),
                    (AckType::NAK, 2),
                    (AckType::ACK, 2),
                    (AckType::ACK, 3)
                ]
            );
            process.await
        })
    }
}
