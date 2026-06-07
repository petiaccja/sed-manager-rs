use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use oneshot::Sender;
use sed_packet::{
    Ignore,
    packet::{PACKET_HEADER_LEN, Packet, SUB_PACKET_HEADER_LEN},
    session_id::SessionId,
};
use sed_spec::methods::{ExtractResult, MethodResult, Properties, extract_method};

use crate::{
    Error,
    protocol::{
        sequence_number::SequenceNumber,
        shared::{eos, packetize_one},
    },
};

#[derive(Debug)]
pub struct Session {
    session_id: SessionId,
    timeout: Duration,
    properties: Properties,
    sequence_number: SequenceNumber,
    state: State,
}

impl Session {
    pub fn new(session_id: SessionId, timeout: Duration, properties: Properties) -> Self {
        Self {
            session_id,
            timeout,
            properties,
            sequence_number: SequenceNumber::initial(),
            state: State::Active {
                method_calls: VecDeque::new(),
                method_calls_sending: VecDeque::new(),
                method_calls_receiving: VecDeque::new(),
                received_tokens: VecDeque::new(),
            },
        }
    }

    pub fn handle_method_call(&mut self, call: Vec<u8>, sender: Sender<Result<Vec<u8>, Error>>) {
        if call.len() > self.properties.max_gross_packet_size.get() - PACKET_HEADER_LEN - SUB_PACKET_HEADER_LEN {
            let _ = sender.send(Err(Error::MethodTooLarge));
        } else {
            match &mut self.state {
                State::Active { method_calls, .. } => {
                    method_calls.push_back(MethodCallRecord { call, sender });
                }
                State::Closed | State::Aborting => {
                    let _ = sender.send(Err(Error::Closed));
                }
            }
        }
    }

    pub fn handle_aborted(&mut self) {
        self.flush(Error::Aborted);
        self.state = State::Closed;
    }

    pub fn handle_iface_send_done(&mut self, time: Instant, sn: SequenceNumber, result: Result<(), Error>) {
        let State::Active { method_calls_sending, method_calls_receiving, .. } = &mut self.state else {
            return;
        };
        while let Some(record) = method_calls_sending.pop_front_if(|record| record.sequence_number <= sn) {
            let deadline = time + self.timeout;
            match &result {
                Ok(_) => {
                    let record = MethodReceivingRecord { deadline, sender: record.sender };
                    method_calls_receiving.push_back(record);
                }
                Err(err) => {
                    let _ = record.sender.send(Err(err.clone()));
                }
            }
        }
    }

    pub fn handle_tokens(&mut self, tokens: Vec<u8>) {
        let State::Active { method_calls_receiving, received_tokens, .. } = &mut self.state else {
            return;
        };
        received_tokens.extend(tokens);
        loop {
            match extract_method::<MethodResult<Vec<Ignore>>>(received_tokens) {
                ExtractResult::Ok { value: _, tokens } => {
                    if let Some(record) = method_calls_receiving.pop_front() {
                        let _ = record.sender.send(Ok(tokens));
                    } else {
                        self.flush(Error::Aborted);
                        self.state = State::Aborting;
                        break;
                    }
                }
                ExtractResult::EndOfSession => {
                    if let Some(record) = method_calls_receiving.pop_front() {
                        let _ = record.sender.send(Ok(eos()));
                    }
                    self.state = State::Closed;
                    break;
                }
                ExtractResult::NeedMoreTokens => break,
                ExtractResult::InvalidTokens(error) => {
                    self.flush(error.into());
                    self.state = State::Aborting;

                    break;
                }
            };
        }
    }

    pub fn poll_action(&mut self, time: Instant) -> Action {
        match &mut self.state {
            State::Active { method_calls, method_calls_sending, method_calls_receiving, .. } => {
                // Remove timed out calls.
                while let Some(record) = method_calls_receiving.pop_front_if(|record| record.deadline <= time) {
                    let _ = record.sender.send(Err(Error::TimedOut));
                }

                // Collect packets ready to be sent.
                let mut packets = Vec::new();
                if let Some(MethodCallRecord { call, sender }) = method_calls.pop_front() {
                    let sn = self.sequence_number.fetch_add();
                    let packet = packetize_one(self.session_id, sn, call);
                    packets.push(packet);
                    method_calls_sending.push_back(MethodSendingRecord { sequence_number: sn, sender });
                }

                // Return next action.
                if !packets.is_empty() {
                    Action::Send(packets)
                } else if let Some(record) = method_calls_receiving.front() {
                    Action::Sleep { until: record.deadline }
                } else {
                    Action::None
                }
            }
            State::Aborting => {
                self.state = State::Closed;
                let eos_packet = packetize_one(self.session_id, self.sequence_number.fetch_add(), eos());
                Action::Send(vec![eos_packet])
            }
            State::Closed => Action::Delete,
        }
    }

    fn flush(&mut self, error: Error) {
        if let State::Active { method_calls, method_calls_receiving, method_calls_sending, received_tokens } =
            &mut self.state
        {
            received_tokens.clear();
            method_calls.drain(..).for_each(|record| {
                let _ = record.sender.send(Err(error.clone()));
            });
            method_calls_sending.drain(..).for_each(|record| {
                let _ = record.sender.send(Err(error.clone()));
            });
            method_calls_receiving.drain(..).for_each(|record| {
                let _ = record.sender.send(Err(error.clone()));
            });
        };
    }
}

#[derive(Debug)]
enum State {
    Active {
        method_calls: VecDeque<MethodCallRecord>,
        /// Method calls that are queued for IF-SEND.
        method_calls_sending: VecDeque<MethodSendingRecord>,
        /// Method calls that are queued for IF-RECV.
        method_calls_receiving: VecDeque<MethodReceivingRecord>,
        received_tokens: VecDeque<u8>,
    },
    Aborting,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub enum Action {
    None,
    Sleep { until: Instant },
    Send(Vec<Packet>),
    Delete,
}

#[derive(Debug)]
pub struct MethodCallRecord {
    call: Vec<u8>,
    sender: Sender<Result<Vec<u8>, Error>>,
}

#[derive(Debug)]
struct MethodSendingRecord {
    /// The sequence number of the packet in which the method is being sent.
    sequence_number: SequenceNumber,
    sender: Sender<Result<Vec<u8>, Error>>,
}

#[derive(Debug)]
struct MethodReceivingRecord {
    /// The time when the message times out.
    deadline: Instant,
    sender: Sender<Result<Vec<u8>, Error>>,
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use googletest::assert_that;
    use googletest::matchers::*;
    use oneshot::TryRecvError;
    use oneshot::channel;
    use sed_packet::packet::{SubPacket, SubPacketKind};

    use super::*;
    use crate::protocol::shared::tests::*;

    const SESSION_ID: SessionId = SessionId { hsn: 1, tsn: 2 };
    const TIMEOUT: Duration = Duration::from_secs(1);
    const PROPERTIES: Properties = Properties::INITIAL;

    #[test]
    fn method_completed_successfully() {
        let mut session = Session::new(SESSION_ID, TIMEOUT, PROPERTIES);
        let time = Instant::now();
        let (sender, receiver) = channel();

        session.handle_method_call(method_call(), sender);
        assert_that!(
            session.poll_action(time),
            matches_pattern!(Action::Send(eq(&vec![Packet {
                tper_session_number: 2,
                host_session_number: 1,
                sequence_number: 1,
                payload: vec![SubPacket { kind: SubPacketKind::Data, length: PhantomData, payload: method_call() }],
                ..Default::default()
            }])))
        );

        session.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        assert_that!(session.poll_action(time), matches_pattern!(&Action::Sleep { until: eq(time + TIMEOUT) }));

        session.handle_tokens(method_response());
        assert_that!(session.poll_action(time), matches_pattern!(&Action::None));
        assert_that!(receiver.try_recv(), ok(ok(eq(&method_response()))));
    }

    #[test]
    fn overlapped_methods_completed_successfully() {
        let mut session = Session::new(SESSION_ID, TIMEOUT, PROPERTIES);
        let time = Instant::now();
        let delay = TIMEOUT / 10;
        let (sender_1, receiver_1) = channel();
        let (sender_2, receiver_2) = channel();

        // Enqueue both methods.
        session.handle_method_call(method_call(), sender_1);
        session.handle_method_call(method_call(), sender_2);

        // Dequeue both associated packets.
        assert_that!(
            session.poll_action(time + 0 * delay),
            matches_pattern!(Action::Send(elements_are![field!(&Packet.sequence_number, 1)]))
        );
        assert_that!(
            session.poll_action(time + 1 * delay),
            matches_pattern!(Action::Send(elements_are![field!(&Packet.sequence_number, 2)]))
        );

        // Notify IF-SEND done for both packets.
        session.handle_iface_send_done(time + 2 * delay, SequenceNumber(1), Ok(()));
        assert_that!(
            session.poll_action(time + 3 * delay),
            matches_pattern!(&Action::Sleep { until: eq(time + TIMEOUT + 2 * delay) })
        );

        session.handle_iface_send_done(time + 4 * delay, SequenceNumber(2), Ok(()));
        assert_that!(
            session.poll_action(time + 5 * delay),
            matches_pattern!(&Action::Sleep { until: eq(time + TIMEOUT + 2 * delay) })
        );

        // Notify incoming tokens for both methods.
        session.handle_tokens(method_response());
        assert_that!(
            session.poll_action(time + 6 * delay),
            matches_pattern!(&Action::Sleep { until: eq(time + TIMEOUT + 4 * delay) })
        );
        assert_that!(receiver_1.try_recv(), ok(ok(eq(&method_response()))));

        session.handle_tokens(method_response());
        assert_that!(session.poll_action(time + 7 * delay), eq(&Action::None));
        assert_that!(receiver_2.try_recv(), ok(ok(eq(&method_response()))));
    }

    #[test]
    fn sequential_methods_completed_successfully() {
        let mut session = Session::new(SESSION_ID, TIMEOUT, PROPERTIES);
        let time = Instant::now();

        for i in 0..1 {
            let time = time + i * TIMEOUT;
            let (sender, receiver) = channel();
            session.handle_method_call(method_call(), sender);
            assert_that!(
                session.poll_action(time),
                matches_pattern!(Action::Send(eq(&vec![Packet {
                    tper_session_number: 2,
                    host_session_number: 1,
                    sequence_number: 1,
                    payload: vec![SubPacket { kind: SubPacketKind::Data, length: PhantomData, payload: method_call() }],
                    ..Default::default()
                }])))
            );

            session.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
            assert_that!(session.poll_action(time), matches_pattern!(&Action::Sleep { until: eq(time + TIMEOUT) }));

            session.handle_tokens(method_response());
            assert_that!(session.poll_action(time), matches_pattern!(&Action::None));
            assert_that!(receiver.try_recv(), ok(ok(eq(&method_response()))));
        }
    }

    #[test]
    fn interface_send_failed() {
        let mut session = Session::new(SESSION_ID, TIMEOUT, PROPERTIES);
        let time = Instant::now();
        let (sender, receiver) = channel();

        session.handle_method_call(method_call(), sender);
        assert_that!(session.poll_action(time), pat!(Action::Send(_)));

        session.handle_iface_send_done(time, SequenceNumber(1), Err(Error::NotSupported));
        assert_that!(session.poll_action(time), pat!(Action::None));
        assert_that!(receiver.try_recv(), ok(err(eq(&Error::NotSupported))));
    }

    #[test]
    fn unexpected_tokens() {
        let mut session = Session::new(SESSION_ID, TIMEOUT, PROPERTIES);
        let time = Instant::now();

        session.handle_tokens(method_response());
        assert_that!(
            session.poll_action(time),
            eq(&Action::Send(vec![Packet {
                tper_session_number: 2,
                host_session_number: 1,
                sequence_number: 1,
                payload: vec![SubPacket { kind: SubPacketKind::Data, length: PhantomData, payload: eos() }],
                ..Default::default()
            }]))
        );
        assert_that!(session.poll_action(time), eq(&Action::Delete));
    }

    #[test]
    fn fragmented_tokens() {
        let mut session = Session::new(SESSION_ID, TIMEOUT, PROPERTIES);
        let time = Instant::now();
        let (sender, receiver) = channel();
        let mut first_tokens = method_response();
        let second_tokens = first_tokens.split_off(2);

        session.handle_method_call(method_call(), sender);
        assert_that!(session.poll_action(time), pat!(&Action::Send(_)));

        session.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        assert_that!(session.poll_action(time), pat!(&Action::Sleep { .. }));

        session.handle_tokens(first_tokens);
        assert_that!(session.poll_action(time), pat!(&Action::Sleep { .. }));
        assert_that!(receiver.try_recv(), err(eq(&TryRecvError::Empty)));

        session.handle_tokens(second_tokens);
        assert_that!(session.poll_action(time), eq(&Action::None));
        assert_that!(receiver.try_recv(), ok(ok(eq(&method_response()))));
    }

    #[test]
    fn invalid_tokens() {
        let mut session = Session::new(SESSION_ID, TIMEOUT, PROPERTIES);
        let time = Instant::now();
        let (sender, receiver) = channel();
        let invalid_tokens = vec![0xFE, 34, 23, 7, 2, 3, 2];

        session.handle_method_call(method_call(), sender);
        assert_that!(session.poll_action(time), pat!(&Action::Send(_)));

        session.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        assert_that!(session.poll_action(time), pat!(&Action::Sleep { .. }));

        session.handle_tokens(invalid_tokens);
        assert_that!(
            session.poll_action(time),
            eq(&Action::Send(vec![Packet {
                tper_session_number: 2,
                host_session_number: 1,
                sequence_number: 2,
                payload: vec![SubPacket { kind: SubPacketKind::Data, length: PhantomData, payload: eos() }],
                ..Default::default()
            }]))
        );
        assert_that!(receiver.try_recv(), ok(err(matches_pattern!(&Error::TokenError(_)))));
        assert_that!(session.poll_action(time), eq(&Action::Delete));
    }

    #[test]
    fn end_session() {
        let mut session = Session::new(SESSION_ID, TIMEOUT, PROPERTIES);
        let time = Instant::now();
        let (sender, receiver) = channel();

        session.handle_method_call(eos(), sender);
        assert_that!(
            session.poll_action(time),
            eq(&Action::Send(vec![Packet {
                tper_session_number: 2,
                host_session_number: 1,
                sequence_number: 1,
                payload: vec![SubPacket { kind: SubPacketKind::Data, length: PhantomData, payload: eos() }],
                ..Default::default()
            }]))
        );

        session.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        session.handle_tokens(eos());
        assert_that!(session.poll_action(time), eq(&Action::Delete));
        assert_that!(receiver.try_recv(), ok(ok(eq(&eos()))));
    }

    #[test]
    fn reject_calls_when_closed() {
        let mut session = Session::new(SESSION_ID, TIMEOUT, PROPERTIES);
        let time = Instant::now();

        session.handle_aborted();

        let (sender, receiver) = channel();
        session.handle_method_call(eos(), sender);
        assert_that!(session.poll_action(time), eq(&Action::Delete));
        assert_that!(receiver.try_recv(), ok(err(eq(&Error::Closed))));
    }

    #[test]
    fn reject_oversized_calls() {
        let mut session = Session::new(SESSION_ID, TIMEOUT, PROPERTIES);
        let time = Instant::now();

        let (sender, receiver) = channel();
        session.handle_method_call(vec![0; 1025], sender);
        assert_that!(session.poll_action(time), eq(&Action::None));
        assert_that!(receiver.try_recv(), ok(err(eq(&Error::MethodTooLarge))));
    }

    #[test]
    fn timeout() {
        let mut session = Session::new(SESSION_ID, TIMEOUT, PROPERTIES);
        let time = Instant::now();
        let (sender, receiver) = channel();

        session.handle_method_call(method_call(), sender);
        assert_that!(session.poll_action(time), matches_pattern!(Action::Send(_)));

        session.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        assert_that!(session.poll_action(time), matches_pattern!(&Action::Sleep { until: eq(time + TIMEOUT) }));

        assert_that!(session.poll_action(time + 2 * TIMEOUT), matches_pattern!(&Action::None));
        assert_that!(receiver.try_recv(), ok(err(eq(&Error::TimedOut))));
    }
}
