use std::{
    cmp::{max, min},
    collections::VecDeque,
    marker::PhantomData,
    time::Duration,
};

use sed_packet::{
    com_id::{COM_ID_RESPONSE_LEN, ComIdRequest, ComIdResponse, ComIdResponsePayload},
    packet::ComPacket,
};
use sorbit::{error::Error as SorbitError, ser_de::ToBytes};
use std::time::Instant;

use crate::{Error, protocol::shared::Action};

const MAX_BACKOFF: Duration = Duration::from_millis(500);
const RECV_FAILURE_BACKOFF: Duration = Duration::from_millis(500);
const INITIAL_BACKOFF: Duration = Duration::from_millis(1);
const MAX_RECV_ATTEMPTS: u64 = 3;

/// A state machine for synchronous interface communications.
///
/// As described in the Core Specification, when using synchronous communication,
/// an IF-SEND command is issued, then the response is retrieved by one or more
/// IF-RECV commands. Another IF-SEND SHALL not be issued before the entire
/// response has been retrieved via IF-RECV commands.
///
/// The synchronous protocol defined in the Core Specification only applies to
/// protocol 0x01 (token stream). While it does not apply to protocol 0x02
/// (ComID requests), protocol 0x02 is just a more relaxed version of this, so
/// it will also work for protocol 0x02.
///
/// Overlapping IF-SEND-IF-RECV cycles on protocol 0x01 and procotol 0x02 is
/// allowed as per the Core Specification
#[derive(Debug)]
pub struct SynchronousProtocol<SendMessage, RecvMessage>
where
    SendMessage: InterfaceMessage + core::fmt::Debug,
    RecvMessage: InterfaceMessage + core::fmt::Debug,
{
    protocol: u8,
    max_transfer_len: usize,
    queue: VecDeque<SendMessage>,
    recv_attempt: u64,
    phase: Phase,
    _recv: PhantomData<RecvMessage>,
}

impl<SendMessage, RecvMessage> SynchronousProtocol<SendMessage, RecvMessage>
where
    SendMessage: InterfaceMessage + core::fmt::Debug,
    RecvMessage: InterfaceMessage + core::fmt::Debug,
{
    pub fn new(protocol: u8, max_transfer_len: usize) -> Self {
        Self {
            protocol,
            max_transfer_len,
            queue: VecDeque::new(),
            recv_attempt: 0,
            phase: Phase::Send,
            _recv: PhantomData,
        }
    }

    pub fn handle_send(&mut self, send_message: SendMessage) {
        self.queue.push_back(send_message);
    }

    pub fn handle_recv(&mut self, time: Instant, result: Result<&RecvMessage, &Error>) {
        match result {
            Ok(message) => self.handle_recv_ok(time, message),
            Err(_) => self.handle_recv_error(time),
        }
    }

    pub fn handle_reset(&mut self) {
        *self = Self::new(self.protocol, self.max_transfer_len)
    }

    fn handle_recv_ok(&mut self, time: Instant, message: &RecvMessage) {
        self.recv_attempt = 0;
        let min_transfer = message.min_transfer();
        let outstanding_data = message.outstanding_data();
        let next_transfer_len = min(self.max_transfer_len, max(min_transfer, outstanding_data));

        let new_phase = match self.phase.clone() {
            Phase::Send => Phase::Send,
            Phase::Receive { backoff, transfer_len, .. }
            | Phase::ReceiveAfter { backoff, transfer_len, .. }
            | Phase::Receiving { backoff, transfer_len } => {
                if outstanding_data == 0 {
                    // If there is no more data to receive, the receive
                    // phase is over, and we can start sending again.
                    Phase::Send
                } else if !message.is_empty() || (message.is_empty() && transfer_len < min_transfer) {
                    // If we got some data, the device might have already
                    // prepared some more for us, so let's do a receive again
                    // without waiting and reset the backoff.
                    //
                    // If we got no data because the transfer length was too
                    // small, we want to receive again immediately.
                    Phase::Receive { transfer_len: next_transfer_len, backoff: INITIAL_BACKOFF }
                } else {
                    // If we got no data, the device might need some time to
                    // prepare the results, so let's wait a bit before polling
                    // it again.
                    let after = time + backoff;
                    let backoff = min(backoff * 2, MAX_BACKOFF);
                    Phase::ReceiveAfter { after, transfer_len: next_transfer_len, backoff }
                }
            }
            Phase::Recovering => Phase::Recovering,
        };

        self.phase = new_phase;
    }

    fn handle_recv_error(&mut self, time: Instant) {
        let new_phase = match self.phase.clone() {
            Phase::Send => Phase::Send,
            Phase::Receive { backoff, transfer_len, .. }
            | Phase::ReceiveAfter { backoff, transfer_len, .. }
            | Phase::Receiving { backoff, transfer_len } => {
                let recv_attempt = self.recv_attempt;
                self.recv_attempt += 1;
                if recv_attempt == 0 {
                    Phase::Receive { transfer_len, backoff }
                } else {
                    Phase::ReceiveAfter { after: time + RECV_FAILURE_BACKOFF, transfer_len, backoff }
                }
            }
            Phase::Recovering => Phase::Recovering,
        };

        self.phase = new_phase;
    }

    pub fn poll_action(&mut self, time: Instant) -> Action {
        let (action, new_phase) = match self.phase.clone() {
            Phase::Send => match self.queue.pop_front() {
                Some(message) => (
                    Action::Send {
                        protocol: self.protocol,
                        data: message.to_bytes_interface().expect("can not serialize message"),
                    },
                    Phase::Receive { transfer_len: RecvMessage::INITIAL_TRANSFER, backoff: INITIAL_BACKOFF },
                ),
                None => (Action::None, Phase::Send),
            },
            Phase::Receive { transfer_len, backoff } => {
                if self.recv_attempt < MAX_RECV_ATTEMPTS {
                    (Action::Recv { protocol: self.protocol, transfer_len }, Phase::Receiving { transfer_len, backoff })
                } else {
                    // In case IF-RECV has failed too many times in a sequence,
                    // the situation is probably unsalvagable, as we have no
                    // idea when and if responses to our packets will arrive.
                    // The only option we have is to issue a stack reset, assume
                    // that we're now in the sending phase, and hope for the
                    // best. Request a recovery and let the higher level protocol
                    // issue the stack reset.
                    self.recv_attempt = 0;
                    (Action::Recover, Phase::Recovering)
                }
            }
            Phase::Receiving { transfer_len, backoff } => (Action::None, Phase::Receiving { transfer_len, backoff }),
            Phase::ReceiveAfter { after, transfer_len, backoff } => {
                if self.recv_attempt < MAX_RECV_ATTEMPTS {
                    if after <= time {
                        (
                            Action::Recv { protocol: self.protocol, transfer_len },
                            Phase::Receiving { transfer_len, backoff },
                        )
                    } else {
                        (Action::Sleep { until: after }, Phase::ReceiveAfter { after, transfer_len, backoff })
                    }
                } else {
                    self.recv_attempt = 0;
                    (Action::Recover, Phase::Recovering)
                }
            }
            Phase::Recovering => (Action::None, Phase::Recovering),
        };

        self.phase = new_phase;
        action
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Phase {
    /// The next action may be a send, if there is something to send.
    Send,
    /// The next action must be a receive.
    Receive { transfer_len: usize, backoff: Duration },
    /// Waiting for the completion of a receive action.
    Receiving { transfer_len: usize, backoff: Duration },
    /// The next action will be receive in the future, but first we need to
    /// sleep.
    ReceiveAfter { after: Instant, transfer_len: usize, backoff: Duration },
    /// When too many failed IF-RECVs get the protocol stack unstable/unknown.
    Recovering,
}

pub trait InterfaceMessage {
    type Error: core::error::Error;
    const INITIAL_TRANSFER: usize;

    fn outstanding_data(&self) -> usize;
    fn min_transfer(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn to_bytes_interface(&self) -> Result<Vec<u8>, Self::Error>;
}

impl InterfaceMessage for ComPacket {
    type Error = SorbitError;
    const INITIAL_TRANSFER: usize = 512;

    fn outstanding_data(&self) -> usize {
        self.min_transfer as usize
    }

    fn min_transfer(&self) -> usize {
        self.outstanding_data as usize
    }

    fn is_empty(&self) -> bool {
        self.payload.is_empty()
    }

    fn to_bytes_interface(&self) -> Result<Vec<u8>, Self::Error> {
        ToBytes::to_bytes(self)
    }
}

impl InterfaceMessage for ComIdRequest {
    type Error = SorbitError;
    const INITIAL_TRANSFER: usize = 8;

    fn outstanding_data(&self) -> usize {
        0
    }

    fn min_transfer(&self) -> usize {
        8
    }

    fn is_empty(&self) -> bool {
        false
    }

    fn to_bytes_interface(&self) -> Result<Vec<u8>, Self::Error> {
        ToBytes::to_bytes(self)
    }
}

impl InterfaceMessage for ComIdResponse {
    type Error = SorbitError;
    const INITIAL_TRANSFER: usize = COM_ID_RESPONSE_LEN;

    fn outstanding_data(&self) -> usize {
        match self.payload {
            ComIdResponsePayload::NoResponseAvailable { .. } => 0,
            ComIdResponsePayload::Verify { .. } => 0,
            ComIdResponsePayload::StackReset { available_data_length, .. } => {
                if available_data_length != 0 {
                    0
                } else {
                    COM_ID_RESPONSE_LEN
                }
            }
        }
    }

    fn min_transfer(&self) -> usize {
        COM_ID_RESPONSE_LEN
    }

    fn is_empty(&self) -> bool {
        match self.payload {
            ComIdResponsePayload::NoResponseAvailable { .. } => false,
            ComIdResponsePayload::Verify { .. } => false,
            ComIdResponsePayload::StackReset { available_data_length, .. } => available_data_length < 4,
        }
    }

    fn to_bytes_interface(&self) -> Result<Vec<u8>, Self::Error> {
        ToBytes::to_bytes(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use googletest::{assert_that, matchers::*};
    use sed_packet::com_id::{COM_ID_PROTOCOL, ComIdRequest, ComIdState, Date, StackResetStatus};
    use sed_packet::packet::{PACKETIZED_PROTOCOL, Packet};

    const COM_ID: u16 = 1;
    const COM_ID_EXT: u16 = 0;

    #[test]
    fn stack_reset_exchanged_with_delay() {
        const REQUEST: ComIdRequest = ComIdRequest::stack_reset(COM_ID, COM_ID_EXT);
        const RESPONSE_PENDING: ComIdResponse = ComIdResponse {
            com_id: 1,
            com_id_ext: 1,
            payload: ComIdResponsePayload::StackReset { available_data_length: 0, status: StackResetStatus::Success },
        };
        const RESPONSE_DONE: ComIdResponse = ComIdResponse {
            com_id: 1,
            com_id_ext: 1,
            payload: ComIdResponsePayload::StackReset { available_data_length: 4, status: StackResetStatus::Success },
        };

        let mut protocol = SynchronousProtocol::new(COM_ID_PROTOCOL, 512);
        let time_0 = Instant::now();

        let sequence = [
            (time_0, Action::Send { protocol: COM_ID_PROTOCOL, data: REQUEST.to_bytes().unwrap() }, None),
            (time_0, Action::Recv { protocol: COM_ID_PROTOCOL, transfer_len: 46 }, None),
            (time_0, Action::None, Some(Ok(RESPONSE_PENDING))),
            (time_0, Action::Sleep { until: time_0 + INITIAL_BACKOFF }, None),
            (time_0 + INITIAL_BACKOFF, Action::Recv { protocol: COM_ID_PROTOCOL, transfer_len: 46 }, None),
            (time_0 + INITIAL_BACKOFF, Action::None, None),
            (time_0 + INITIAL_BACKOFF, Action::None, Some(Ok(RESPONSE_PENDING))),
            (time_0 + INITIAL_BACKOFF, Action::Sleep { until: time_0 + 3 * INITIAL_BACKOFF }, None),
            (time_0 + 3 * INITIAL_BACKOFF, Action::Recv { protocol: COM_ID_PROTOCOL, transfer_len: 46 }, None),
            (time_0 + 3 * INITIAL_BACKOFF, Action::None, None),
            (time_0 + 3 * INITIAL_BACKOFF, Action::None, Some(Ok(RESPONSE_DONE))),
            (time_0 + 3 * INITIAL_BACKOFF, Action::None, None),
        ];

        run_sequence(&mut protocol, time_0, REQUEST, &sequence);
    }

    #[test]
    fn verify_com_id_exchanged() {
        const REQUEST: ComIdRequest = ComIdRequest::verify_com_id_valid(1, 0);
        const RESPONSE: ComIdResponse = ComIdResponse {
            com_id: 1,
            com_id_ext: 1,
            payload: ComIdResponsePayload::Verify {
                available_data_length: 22,
                com_id_state: ComIdState::Associated,
                time_of_allocation: Date::unsupported(),
                time_of_expiry: Date::unsupported(),
                time_since_reset: Date::unsupported(),
            },
        };

        let mut protocol = SynchronousProtocol::new(COM_ID_PROTOCOL, 512);
        let time_0 = Instant::now();

        // The reponse for "verify ComID valid" is always returned immediately.
        let sequence = [
            (time_0, Action::Send { protocol: COM_ID_PROTOCOL, data: REQUEST.to_bytes().unwrap() }, None),
            (time_0, Action::Recv { protocol: COM_ID_PROTOCOL, transfer_len: 46 }, None),
            (time_0, Action::None, Some(Ok(RESPONSE))),
            (time_0, Action::None, None),
        ];

        run_sequence(&mut protocol, time_0, REQUEST, &sequence);
    }

    #[test]
    fn interrupted_with_no_response_available() {
        const REQUEST: ComIdRequest = ComIdRequest::verify_com_id_valid(1, 0);
        const RESPONSE: ComIdResponse = ComIdResponse {
            com_id: 1,
            com_id_ext: 1,
            payload: ComIdResponsePayload::NoResponseAvailable { available_data_length: 0 },
        };

        let mut protocol = SynchronousProtocol::new(COM_ID_PROTOCOL, 512);
        let time_0 = Instant::now();

        // The reponse for "verify ComID valid" is always returned immediately.
        let sequence = [
            (time_0, Action::Send { protocol: COM_ID_PROTOCOL, data: REQUEST.to_bytes().unwrap() }, None),
            (time_0, Action::Recv { protocol: COM_ID_PROTOCOL, transfer_len: 46 }, None),
            (time_0, Action::None, Some(Ok(RESPONSE))),
            (time_0, Action::None, None),
        ];

        run_sequence(&mut protocol, time_0, REQUEST, &sequence);
    }

    #[test]
    fn com_packet_exchanged_with_delay() {
        let request = ComPacket::default();
        let response_pending = ComPacket { outstanding_data: 1, min_transfer: 279, ..Default::default() };
        let response_done = ComPacket { outstanding_data: 0, min_transfer: 0, ..Default::default() };
        let protocol = PACKETIZED_PROTOCOL;
        let time_0 = Instant::now();

        let sequence = [
            (time_0, Action::Send { protocol, data: request.to_bytes().unwrap() }, None),
            (time_0, Action::Recv { protocol, transfer_len: 512 }, None),
            (time_0, Action::None, Some(Ok(response_pending.clone()))),
            (time_0, Action::Sleep { until: time_0 + INITIAL_BACKOFF }, None),
            (time_0 + INITIAL_BACKOFF, Action::Recv { protocol, transfer_len: 279 }, None),
            (time_0 + INITIAL_BACKOFF, Action::None, None),
            (time_0 + INITIAL_BACKOFF, Action::None, Some(Ok(response_pending.clone()))),
            (time_0 + INITIAL_BACKOFF, Action::Sleep { until: time_0 + 3 * INITIAL_BACKOFF }, None),
            (time_0 + 3 * INITIAL_BACKOFF, Action::Recv { protocol, transfer_len: 279 }, None),
            (time_0 + 3 * INITIAL_BACKOFF, Action::None, None),
            (time_0 + 3 * INITIAL_BACKOFF, Action::None, Some(Ok(response_done))),
            (time_0 + 3 * INITIAL_BACKOFF, Action::None, None),
        ];

        let mut protocol = SynchronousProtocol::new(protocol, 16384);
        run_sequence(&mut protocol, time_0, request, &sequence);
    }

    #[test]
    fn com_packet_exchanged_fragmented() {
        let request = ComPacket::default();
        let response_one = ComPacket {
            outstanding_data: 652,
            min_transfer: 279,
            payload: vec![Packet::default()],
            ..Default::default()
        };
        let response_two =
            ComPacket { outstanding_data: 0, min_transfer: 0, payload: vec![Packet::default()], ..Default::default() };
        let protocol = PACKETIZED_PROTOCOL;
        let time_0 = Instant::now();

        let sequence = [
            (time_0, Action::Send { protocol, data: request.to_bytes().unwrap() }, None),
            (time_0, Action::Recv { protocol, transfer_len: 512 }, None),
            (time_0, Action::None, Some(Ok(response_one.clone()))),
            (time_0, Action::Recv { protocol, transfer_len: 652 }, None),
            (time_0, Action::None, Some(Ok(response_two.clone()))),
            (time_0, Action::None, None),
        ];

        let mut protocol = SynchronousProtocol::new(protocol, 16384);
        run_sequence(&mut protocol, time_0, request, &sequence);
    }

    #[test]
    fn com_packet_exchanged_tansfer_len_too_small() {
        let request = ComPacket::default();
        let response_inform = ComPacket { outstanding_data: 2846, min_transfer: 2846, ..Default::default() };
        let response_payload = ComPacket { outstanding_data: 0, min_transfer: 0, ..Default::default() };
        let protocol = PACKETIZED_PROTOCOL;
        let time_0 = Instant::now();

        let sequence = [
            (time_0, Action::Send { protocol, data: request.to_bytes().unwrap() }, None),
            (time_0, Action::Recv { protocol, transfer_len: 512 }, None),
            (time_0, Action::None, Some(Ok(response_inform.clone()))),
            (time_0, Action::Recv { protocol, transfer_len: 2846 }, None),
            (time_0, Action::None, Some(Ok(response_payload.clone()))),
            (time_0, Action::None, None),
        ];

        let mut protocol = SynchronousProtocol::new(protocol, 16384);
        run_sequence(&mut protocol, time_0, request, &sequence);
    }

    // The logic is the same for ComID requests, no point testing that separately.
    #[test]
    fn com_packet_recv_failed_attempts_recovered() {
        let request = ComPacket::default();
        let response = ComPacket { outstanding_data: 0, min_transfer: 0, ..Default::default() };
        let protocol = PACKETIZED_PROTOCOL;
        let time_0 = Instant::now();
        let delay = RECV_FAILURE_BACKOFF * 2;

        let sequence = [
            (time_0, Action::Send { protocol, data: request.to_bytes().unwrap() }, None),
            // Attempt 0
            (time_0, Action::Recv { protocol, transfer_len: 512 }, None),
            (time_0, Action::None, Some(Err(Error::NotSupported))),
            // Attempt 1
            (time_0 + 1 * delay, Action::Recv { protocol, transfer_len: 512 }, None),
            (time_0 + 1 * delay, Action::None, Some(Err(Error::NotSupported))),
            (time_0 + 1 * delay, Action::Sleep { until: time_0 + 1 * delay + RECV_FAILURE_BACKOFF }, None),
            // Attempt 2
            (time_0 + 2 * delay, Action::Recv { protocol, transfer_len: 512 }, None),
            (time_0 + 2 * delay, Action::None, Some(Ok(response))),
            (time_0 + 2 * delay, Action::None, None),
        ];

        let mut protocol = SynchronousProtocol::new(protocol, 16384);
        run_sequence(&mut protocol, time_0, request, &sequence);
        assert_that!(protocol.phase, pat!(Phase::Send));
    }

    // The logic is the same for ComID requests, no point testing that separately.
    #[test]
    fn com_packet_recv_failed_attempts_exceeded() {
        let request = ComPacket::default();
        let protocol = PACKETIZED_PROTOCOL;
        let time_0 = Instant::now();
        let delay = RECV_FAILURE_BACKOFF * 2;

        let sequence = [
            (time_0, Action::Send { protocol, data: request.to_bytes().unwrap() }, None),
            // Attempt 0
            (time_0, Action::Recv { protocol, transfer_len: 512 }, None),
            (time_0, Action::None, Some(Err::<ComPacket, _>(Error::NotSupported))),
            // Attempt 1
            (time_0 + 1 * delay, Action::Recv { protocol, transfer_len: 512 }, None),
            (time_0 + 1 * delay, Action::None, Some(Err(Error::NotSupported))),
            (time_0 + 1 * delay, Action::Sleep { until: time_0 + 1 * delay + RECV_FAILURE_BACKOFF }, None),
            // Attempt 2
            (time_0 + 2 * delay, Action::Recv { protocol, transfer_len: 512 }, None),
            (time_0 + 2 * delay, Action::None, Some(Err(Error::NotSupported))),
            // Failed
            (time_0 + 3 * delay, Action::Recover, None),
            (time_0 + 3 * delay, Action::None, None),
        ];

        let mut protocol = SynchronousProtocol::new(protocol, 16384);
        run_sequence(&mut protocol, time_0, request, &sequence);
        assert_that!(protocol.phase, pat!(Phase::Recovering));
    }

    fn run_sequence<SendMessage, RecvMessage>(
        protocol: &mut SynchronousProtocol<SendMessage, RecvMessage>,
        time_0: Instant,
        request: SendMessage,
        sequence: &[(Instant, Action, Option<Result<RecvMessage, Error>>)],
    ) where
        SendMessage: InterfaceMessage + core::fmt::Debug,
        RecvMessage: InterfaceMessage + core::fmt::Debug,
    {
        protocol.handle_send(request);

        for (step, (time, expected_action, received_data)) in sequence.into_iter().enumerate() {
            let action = protocol.poll_action(*time);
            assert_eq!(&action, expected_action, "step = {}, time = {:?}", step, *time - time_0);
            if let Some(received_data) = received_data {
                protocol.handle_recv(*time, received_data.as_ref());
            }
        }
    }
}
