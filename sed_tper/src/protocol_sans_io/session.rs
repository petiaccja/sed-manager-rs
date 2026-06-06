use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use oneshot::Sender;
use sed_packet::{
    Ignore,
    packet::{PACKET_HEADER_LEN, Packet, SUB_PACKET_HEADER_LEN},
    session_id::SessionId,
    token::{Command, ToTokens},
};
use sed_spec::methods::{ExtractResult, MethodResult, Properties, extract_method};

use crate::{
    Error,
    protocol_sans_io::{sequence_number::SequenceNumber, utility::packetize_one},
};

#[derive(Debug)]
pub struct Session {
    session_id: SessionId,
    timeout: Duration,
    properties: Properties,
    sequence_number: SequenceNumber,
    state: SessionState,
}

impl Session {
    pub fn new(session_id: SessionId, timeout: Duration, properties: Properties) -> Self {
        Self {
            session_id,
            timeout,
            properties,
            sequence_number: SequenceNumber::initial(),
            state: SessionState::Active {
                method_calls_sending: VecDeque::new(),
                method_calls_receiving: VecDeque::new(),
                received_tokens: VecDeque::new(),
            },
        }
    }

    pub fn handle_method_call(&mut self, call: Vec<u8>, sender: Sender<Result<Vec<u8>, Error>>) -> Option<Packet> {
        if call.len() > self.properties.max_gross_packet_size.get() - PACKET_HEADER_LEN - SUB_PACKET_HEADER_LEN {
            let _ = sender.send(Err(Error::MethodTooLarge));
            return None;
        }
        match &mut self.state {
            SessionState::Active { method_calls_sending, .. } => {
                let sequence_number = self.sequence_number.fetch_add();
                method_calls_sending.push_back(MethodSendingRecord { sequence_number, sender });
                Some(packetize_one(self.session_id, sequence_number, call))
            }
            SessionState::Closed => {
                let _ = sender.send(Err(Error::Closed));
                None
            }
        }
    }

    pub fn handle_iface_send_done(&mut self, time: Instant, sn: SequenceNumber, result: Result<(), Error>) {
        let SessionState::Active { method_calls_sending, method_calls_receiving, .. } = &mut self.state else {
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

    pub fn handle_tokens(&mut self, tokens: Vec<u8>) -> SessionAction {
        let SessionState::Active { method_calls_receiving, received_tokens, .. } = &mut self.state else {
            return SessionAction::None;
        };
        received_tokens.extend(tokens);
        loop {
            match extract_method::<MethodResult<Vec<Ignore>>>(received_tokens) {
                ExtractResult::Ok { value: _, tokens } => {
                    if let Some(record) = method_calls_receiving.pop_front() {
                        let _ = record.sender.send(Ok(tokens));
                    } else {
                        self.flush(Error::Aborted);
                        self.state = SessionState::Closed;
                        let sn = self.sequence_number.fetch_add();
                        let packet = packetize_one(self.session_id, sn, eos());
                        break SessionAction::Delete(Some(packet));
                    }
                }
                ExtractResult::EndOfSession => {
                    if let Some(record) = method_calls_receiving.pop_front() {
                        let _ = record.sender.send(Ok(eos()));
                    }
                    break SessionAction::Delete(None);
                }
                ExtractResult::NeedMoreTokens => break SessionAction::None,
                ExtractResult::InvalidTokens(error) => {
                    self.flush(error.into());
                    self.state = SessionState::Closed;
                    let sn = self.sequence_number.fetch_add();
                    let packet = packetize_one(self.session_id, sn, eos());
                    break SessionAction::Delete(Some(packet));
                }
            };
        }
    }

    pub fn poll_timeout(&self) -> Option<Instant> {
        if let SessionState::Active { method_calls_receiving, .. } = &self.state {
            method_calls_receiving.front().map(|record| record.deadline)
        } else {
            None
        }
    }

    pub fn notify_time(&mut self, time: Instant) -> SessionAction {
        let SessionState::Active { method_calls_receiving, .. } = &mut self.state else {
            return SessionAction::None;
        };
        if let Some(record) = method_calls_receiving.pop_front_if(|r| r.deadline < time) {
            let _ = record.sender.send(Err(Error::TimedOut));
            self.flush(Error::TimedOut);
            self.state = SessionState::Closed;
            let sn = self.sequence_number.fetch_add();
            let packet = packetize_one(self.session_id, sn, eos());
            return SessionAction::Delete(Some(packet));
        } else {
            return SessionAction::None;
        }
    }

    pub fn notify_abort(&mut self) {
        self.flush(Error::Aborted);
        self.state = SessionState::Closed;
    }

    fn flush(&mut self, error: Error) {
        if let SessionState::Active { method_calls_receiving, method_calls_sending, received_tokens } = &mut self.state
        {
            received_tokens.clear();
            method_calls_sending.drain(..).for_each(|record| {
                let _ = record.sender.send(Err(error.clone()));
            });
            method_calls_receiving.drain(..).for_each(|record| {
                let _ = record.sender.send(Err(error.clone()));
            });
        };
    }
}

fn eos() -> Vec<u8> {
    Command::EndOfSession.to_tokens().expect("can not serialize EOS command")
}

#[derive(Debug)]
enum SessionState {
    Active {
        /// Method calls that are queued for IF-SEND.
        method_calls_sending: VecDeque<MethodSendingRecord>,
        /// Method calls that are queued for IF-RECV.
        method_calls_receiving: VecDeque<MethodReceivingRecord>,
        received_tokens: VecDeque<u8>,
    },
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub enum SessionAction {
    None,
    Delete(Option<Packet>),
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
    use sed_packet::Bytes;
    use sed_packet::packet::{SubPacket, SubPacketKind};
    use sed_spec::{
        methods::{Random, RandomResult, SessionMethodParam},
        preconfig::core::shared::invoking_id::THIS_SP,
    };

    use super::*;

    const SESSION_ID: SessionId = SessionId { hsn: 1, tsn: 2 };
    const TIMEOUT: Duration = Duration::from_secs(1);
    const PROPERTIES: Properties = Properties::INITIAL;

    fn method_call() -> Vec<u8> {
        Random { count: 4, buffer_out: None }.to_call(THIS_SP).to_tokens().unwrap()
    }

    fn method_response() -> Vec<u8> {
        MethodResult(Ok(RandomResult { result: Bytes(vec![1, 2, 3]) })).to_tokens().unwrap()
    }

    #[test]
    fn method_completed_successfully() {
        let mut session = Session::new(SESSION_ID, TIMEOUT, PROPERTIES);
        let time = Instant::now();
        let (sender, receiver) = channel();
        let packet = session.handle_method_call(method_call(), sender);
        assert_that!(
            packet,
            some(eq(&Packet {
                tper_session_number: 2,
                host_session_number: 1,
                sequence_number: 1,
                payload: vec![SubPacket { kind: SubPacketKind::Data, length: PhantomData, payload: method_call() }],
                ..Default::default()
            }))
        );
        session.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        let action = session.handle_tokens(method_response());
        assert_that!(action, eq(&SessionAction::None));
        assert_that!(receiver.try_recv(), ok(ok(eq(&method_response()))));
    }

    #[test]
    fn overlapped_methods_completed_successfully() {
        let mut session = Session::new(SESSION_ID, TIMEOUT, PROPERTIES);
        let time = Instant::now();
        let (sender_1, receiver_1) = channel();
        let (sender_2, receiver_2) = channel();

        let packet_1 = session.handle_method_call(method_call(), sender_1);
        let packet_2 = session.handle_method_call(method_call(), sender_2);

        assert_that!(packet_1.unwrap().sequence_number, eq(1));
        assert_that!(packet_2.unwrap().sequence_number, eq(2));

        session.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        let action = session.handle_tokens(method_response());
        assert_that!(action, eq(&SessionAction::None));
        assert_that!(receiver_1.try_recv(), ok(ok(eq(&method_response()))));

        session.handle_iface_send_done(time, SequenceNumber(2), Ok(()));
        let action = session.handle_tokens(method_response());
        assert_that!(action, eq(&SessionAction::None));
        assert_that!(receiver_2.try_recv(), ok(ok(eq(&method_response()))));
    }

    #[test]
    fn sequential_methods_completed_successfully() {
        let mut session = Session::new(SESSION_ID, TIMEOUT, PROPERTIES);
        let time = Instant::now();

        let (sender_1, receiver_1) = channel();
        let packet_1 = session.handle_method_call(method_call(), sender_1);
        assert_that!(packet_1.unwrap().sequence_number, eq(1));
        session.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        let action = session.handle_tokens(method_response());
        assert_that!(action, eq(&SessionAction::None));
        assert_that!(receiver_1.try_recv(), ok(ok(eq(&method_response()))));

        let (sender_2, receiver_2) = channel();
        let packet_2 = session.handle_method_call(method_call(), sender_2);
        assert_that!(packet_2.unwrap().sequence_number, eq(2));
        session.handle_iface_send_done(time, SequenceNumber(2), Ok(()));
        let action = session.handle_tokens(method_response());
        assert_that!(action, eq(&SessionAction::None));
        assert_that!(receiver_2.try_recv(), ok(ok(eq(&method_response()))));
    }

    #[test]
    fn interface_send_failed() {
        let mut session = Session::new(SESSION_ID, TIMEOUT, PROPERTIES);
        let time = Instant::now();

        let (sender, receiver) = channel();
        let _packet = session.handle_method_call(method_call(), sender);
        session.handle_iface_send_done(time, SequenceNumber(1), Err(Error::NotSupported));
        assert_that!(receiver.try_recv(), ok(err(eq(&Error::NotSupported))));
    }

    #[test]
    fn unexpected_tokens() {
        let mut session = Session::new(SESSION_ID, TIMEOUT, PROPERTIES);

        let action = session.handle_tokens(method_response());
        assert_that!(
            action,
            eq(&SessionAction::Delete(Some(Packet {
                tper_session_number: 2,
                host_session_number: 1,
                sequence_number: 1,
                payload: vec![SubPacket { kind: SubPacketKind::Data, length: PhantomData, payload: eos() }],
                ..Default::default()
            })))
        );
    }

    #[test]
    fn fragmented_tokens() {
        let mut session = Session::new(SESSION_ID, TIMEOUT, PROPERTIES);
        let time = Instant::now();
        let (sender, receiver) = channel();
        let mut first_tokens = method_response();
        let second_tokens = first_tokens.split_off(2);

        let _packet = session.handle_method_call(method_call(), sender);
        session.handle_iface_send_done(time, SequenceNumber(1), Ok(()));

        let action = session.handle_tokens(first_tokens);
        assert_that!(action, eq(&SessionAction::None));
        assert_that!(receiver.try_recv(), err(eq(&TryRecvError::Empty)));

        let action = session.handle_tokens(second_tokens);
        assert_that!(action, eq(&SessionAction::None));
        assert_that!(receiver.try_recv(), ok(ok(eq(&method_response()))));
    }

    #[test]
    fn invalid_tokens() {
        let mut session = Session::new(SESSION_ID, TIMEOUT, PROPERTIES);
        let time = Instant::now();
        let (sender, receiver) = channel();
        let invalid_tokens = vec![0xFE, 34, 23, 7, 2, 3, 2];

        let _packet = session.handle_method_call(method_call(), sender);
        session.handle_iface_send_done(time, SequenceNumber(1), Ok(()));

        let action = session.handle_tokens(invalid_tokens);
        assert_that!(
            action,
            eq(&SessionAction::Delete(Some(Packet {
                tper_session_number: 2,
                host_session_number: 1,
                sequence_number: 2,
                payload: vec![SubPacket { kind: SubPacketKind::Data, length: PhantomData, payload: eos() }],
                ..Default::default()
            })))
        );
        assert_that!(receiver.try_recv(), ok(err(matches_pattern!(&Error::TokenError(_)))));
    }

    #[test]
    fn end_session() {
        let mut session = Session::new(SESSION_ID, TIMEOUT, PROPERTIES);
        let time = Instant::now();
        let (sender, receiver) = channel();
        let packet = session.handle_method_call(eos(), sender);
        assert_that!(
            packet,
            some(eq(&Packet {
                tper_session_number: 2,
                host_session_number: 1,
                sequence_number: 1,
                payload: vec![SubPacket { kind: SubPacketKind::Data, length: PhantomData, payload: eos() }],
                ..Default::default()
            }))
        );
        session.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        let action = session.handle_tokens(eos());
        assert_that!(action, eq(&SessionAction::Delete(None)));
        assert_that!(receiver.try_recv(), ok(ok(eq(&eos()))));
    }

    #[test]
    fn reject_calls_when_closed() {
        let mut session = Session::new(SESSION_ID, TIMEOUT, PROPERTIES);
        session.notify_abort();

        let (sender, receiver) = channel();
        let packet = session.handle_method_call(eos(), sender);
        assert_that!(packet, none());
        assert_that!(receiver.try_recv(), ok(err(eq(&Error::Closed))));
    }

    #[test]
    fn reject_oversized_calls() {
        let mut session = Session::new(SESSION_ID, TIMEOUT, PROPERTIES);

        let (sender, receiver) = channel();
        let packet = session.handle_method_call(vec![0; 1025], sender);
        assert_that!(packet, none());
        assert_that!(receiver.try_recv(), ok(err(eq(&Error::MethodTooLarge))));
    }

    #[test]
    fn timeout() {
        let mut session = Session::new(SESSION_ID, TIMEOUT, PROPERTIES);
        let time = Instant::now();

        let (sender, receiver) = channel();
        let _packet = session.handle_method_call(method_call(), sender);
        session.handle_iface_send_done(time, SequenceNumber(1), Ok(()));

        let deadline = session.poll_timeout();
        assert_that!(deadline, some(eq(time + TIMEOUT)));

        let action = session.notify_time(time + TIMEOUT / 2);
        assert_that!(action, eq(&SessionAction::None));
        assert_that!(receiver.try_recv(), err(eq(&TryRecvError::Empty)));

        let action = session.notify_time(time + TIMEOUT + TIMEOUT / 2);
        assert_that!(
            action,
            eq(&SessionAction::Delete(Some(Packet {
                tper_session_number: 2,
                host_session_number: 1,
                sequence_number: 2,
                payload: vec![SubPacket { kind: SubPacketKind::Data, length: PhantomData, payload: eos() }],
                ..Default::default()
            })))
        );
        assert_that!(receiver.try_recv(), ok(err(eq(&Error::TimedOut))));
    }
}
