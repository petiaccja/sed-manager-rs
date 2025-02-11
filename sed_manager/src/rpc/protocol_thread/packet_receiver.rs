use std::collections::VecDeque;
use std::time::Instant;
use tokio::sync::oneshot;

use crate::messaging::packet::{Packet, SubPacketKind};
use crate::messaging::token::{DeserializeTokens, Tag, Token};
use crate::rpc::{Error, PackagedMethod, Properties};
use crate::serialization::vec_without_len::VecWithoutLen;
use crate::serialization::DeserializeBinary;

type Response = Result<PackagedMethod, Error>;
type Promise = oneshot::Sender<Response>;

pub struct PacketReceiver {
    deserializer: Deserializer,
    splitter: Splitter,
    timer: Timer,
    promise_buffer: VecDeque<Promise>,
}

struct Deserializer {
    packet_buffer: VecDeque<Packet>,
    token_buffer: VecDeque<Token>,
    error: Option<Error>,
}

struct Splitter {
    token_buffer: VecDeque<Token>,
    method_tokens: Vec<Token>,
    state: SplitterState,
    method_buffer: VecDeque<Result<PackagedMethod, Error>>,
}

struct Timer {
    properties: Properties,
    method_buffer: VecDeque<Result<PackagedMethod, Error>>,
    last_method_time: Instant,
    error: Option<Error>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SplitterState {
    GatheringInvocation,
    GatheringResult(usize),
    Done,
}

impl PacketReceiver {
    pub fn new(properties: Properties) -> Self {
        Self {
            deserializer: Deserializer::new(),
            splitter: Splitter::new(),
            timer: Timer::new(properties),
            promise_buffer: VecDeque::new(),
        }
    }

    pub fn enqueue_promise(&mut self, promise: Promise) {
        self.promise_buffer.push_back(promise);
    }

    pub fn enqueue_packet(&mut self, packet: Packet) {
        self.deserializer.enqueue(packet);
    }

    pub fn has_pending(&self) -> bool {
        !self.promise_buffer.is_empty()
    }

    pub fn poll(&mut self) -> Option<(Promise, Response)> {
        while let Some(maybe_token) = self.deserializer.poll() {
            match maybe_token {
                Ok(token) => self.splitter.enqueue(token),
                Err(err) => return self.promise_buffer.pop_front().map(|promise| (promise, Err(err))),
            }
        }
        while let Some(method) = self.splitter.poll() {
            self.timer.enqueue_method(method);
        }
        if !self.promise_buffer.is_empty() {
            if let Some(result) = self.timer.poll() {
                Some((self.promise_buffer.pop_front().unwrap(), result))
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl Deserializer {
    pub fn new() -> Self {
        Self { packet_buffer: VecDeque::new(), token_buffer: VecDeque::new(), error: None }
    }

    pub fn enqueue(&mut self, packet: Packet) {
        self.packet_buffer.push_back(packet);
    }

    pub fn poll(&mut self) -> Option<Result<Token, Error>> {
        if self.error.is_none() {
            self.process();
        } else {
            self.packet_buffer.clear();
        }

        if let Some(token) = self.token_buffer.pop_front() {
            Some(Ok(token))
        } else if let Some(error) = &self.error {
            Some(Err(error.clone()))
        } else {
            None
        }
    }

    fn process(&mut self) {
        while let Some(packet) = self.packet_buffer.pop_front() {
            packet
                .payload
                .into_vec()
                .into_iter()
                .filter(|sub_packet| sub_packet.kind == SubPacketKind::Data)
                .map(move |sub_packet| VecWithoutLen::<Token>::from_bytes(sub_packet.payload.into_vec()))
                .map_while(|maybe_tokens| match maybe_tokens {
                    Ok(tokens) => Some(tokens.into_vec()),
                    Err(err) => {
                        self.error.replace(Error::SerializationFailed(err));
                        None
                    }
                })
                .flatten()
                .for_each(|token| self.token_buffer.push_back(token));
        }
    }
}

impl Splitter {
    pub fn new() -> Self {
        Self {
            method_buffer: VecDeque::new(),
            method_tokens: Vec::new(),
            token_buffer: VecDeque::new(),
            state: SplitterState::GatheringInvocation,
        }
    }

    pub fn enqueue(&mut self, token: Token) {
        self.token_buffer.push_back(token);
    }

    pub fn poll(&mut self) -> Option<Result<PackagedMethod, Error>> {
        self.process();
        self.method_buffer.pop_front()
    }

    fn process(&mut self) {
        while let Some(token) = self.token_buffer.pop_front() {
            self.state = Self::update_state(self.state, token.tag);
            self.method_tokens.push(token);
            if self.state == SplitterState::Done {
                self.state = SplitterState::GatheringInvocation;
                self.commit_method();
            }
        }
    }

    fn update_state(state: SplitterState, tag: Tag) -> SplitterState {
        match state {
            SplitterState::GatheringInvocation => match tag {
                Tag::EndOfData => SplitterState::GatheringResult(0),
                Tag::EndOfSession => SplitterState::Done,
                _ => state,
            },
            SplitterState::GatheringResult(depth) => {
                let new_depth = match tag {
                    Tag::StartList => depth + 1,
                    Tag::EndList => std::cmp::max(1, depth) - 1,
                    _ => depth,
                };
                match new_depth {
                    0 => SplitterState::Done,
                    _ => SplitterState::GatheringResult(new_depth),
                }
            }
            SplitterState::Done => panic!("you should commit the method call and reset the state instead"),
        }
    }

    fn commit_method(&mut self) {
        let maybe_pm = PackagedMethod::from_tokens(std::mem::replace(&mut self.method_tokens, Vec::new()));
        self.method_buffer.push_back(maybe_pm.map_err(|err| Error::TokenizationFailed(err)));
    }
}

impl Timer {
    pub fn new(properties: Properties) -> Self {
        Self { properties, method_buffer: VecDeque::new(), last_method_time: Instant::now(), error: None }
    }

    pub fn enqueue_method(&mut self, method: Result<PackagedMethod, Error>) {
        self.last_method_time = Instant::now();
        self.method_buffer.push_back(method);
    }

    pub fn poll(&mut self) -> Option<Result<PackagedMethod, Error>> {
        let elapsed = Instant::now() - self.last_method_time;

        if let Some(error) = &self.error {
            self.method_buffer.clear();
            Some(Err(error.clone()))
        } else if let Some(response) = self.method_buffer.pop_front() {
            Some(response)
        } else if elapsed > self.properties.trans_timeout {
            let response = Err(Error::TimedOut);
            self.error = Some(Error::AbortedByHost);
            Some(response)
        } else {
            None
        }
    }
}

impl Drop for PacketReceiver {
    fn drop(&mut self) {
        for promise in std::mem::replace(&mut self.promise_buffer, VecDeque::new()) {
            let _ = promise.send(Err(Error::AbortedByHost));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::messaging::uid::UID;
    use crate::messaging::{packet::SubPacket, value::Value};
    use crate::rpc::{MethodCall, MethodResult, MethodStatus};
    use crate::serialization::{OutputStream, Serialize};

    use super::*;

    #[test]
    fn deserializer_empty() {
        let mut deserializer = Deserializer::new();
        assert!(deserializer.poll().is_none());
    }

    #[test]
    fn deserializer_some() {
        let mut deserializer = Deserializer::new();
        let packet = Packet {
            payload: vec![SubPacket { kind: SubPacketKind::Data, payload: vec![Tag::EndOfSession as u8].into() }]
                .into(),
            ..Default::default()
        };
        deserializer.enqueue(packet);
        assert_eq!(deserializer.poll(), Some(Ok(Token { tag: Tag::EndOfSession, ..Default::default() })));
        assert_eq!(deserializer.poll(), None);
    }

    #[test]
    fn deserializer_error() {
        let mut deserializer = Deserializer::new();
        let packet = Packet {
            payload: vec![SubPacket { kind: SubPacketKind::Data, payload: vec![0xFD].into() }].into(),
            ..Default::default()
        };
        deserializer.enqueue(packet);
        assert!(deserializer.poll().is_some_and(|result| result.is_err()));
        assert!(deserializer.poll().is_some_and(|result| result.is_err())); // Error should be repeated.
    }

    #[test]
    fn deserializer_ignore_credit_control() {
        let mut deserializer = Deserializer::new();
        let packet = Packet {
            payload: vec![SubPacket { kind: SubPacketKind::CreditControl, payload: vec![0, 0, 0, 100].into() }].into(),
            ..Default::default()
        };
        deserializer.enqueue(packet);
        assert_eq!(deserializer.poll(), None);
    }

    #[test]
    fn splitter_empty() {
        let mut splitter = Splitter::new();
        assert!(splitter.poll().is_none());
    }

    #[test]
    fn splitter_some() {
        let items = [
            PackagedMethod::Result(MethodResult { results: vec![Value::from(234_u16)], status: MethodStatus::Fail }),
            PackagedMethod::EndOfSession,
            PackagedMethod::Call(MethodCall {
                args: vec![],
                invoking_id: UID::from(34u64),
                method_id: UID::from(23u64),
                status: MethodStatus::Fail,
            }),
        ];
        let mut stream = OutputStream::<Token>::new();
        items[0].serialize(&mut stream).unwrap();
        items[1].serialize(&mut stream).unwrap();
        items[2].serialize(&mut stream).unwrap();

        let mut splitter = Splitter::new();
        for token in stream.take() {
            splitter.enqueue(token);
        }
        assert_eq!(splitter.poll(), Some(Ok(items[0].clone())));
        assert_eq!(splitter.poll(), Some(Ok(items[1].clone())));
        assert_eq!(splitter.poll(), Some(Ok(items[2].clone())));
        assert_eq!(splitter.poll(), None);
    }

    #[test]
    fn splitter_invalid_split() {
        let tokens = [
            Token { tag: Tag::StartList, ..Default::default() },
            Token { tag: Tag::EndList, ..Default::default() },
            Token { tag: Tag::EndOfData, ..Default::default() },
            Token { tag: Tag::EndList, ..Default::default() },
            Token { tag: Tag::EndOfSession, ..Default::default() },
        ];

        let mut splitter = Splitter::new();
        for token in tokens {
            splitter.enqueue(token);
        }
        assert!(splitter.poll().is_some_and(|result| result.is_err()));
        assert_eq!(splitter.poll(), Some(Ok(PackagedMethod::EndOfSession)));
        assert_eq!(splitter.poll(), None);
    }

    #[test]
    fn splitter_invalid_format() {
        let tokens = [
            Token { tag: Tag::StartList, ..Default::default() },
            Token { tag: Tag::EndList, ..Default::default() },
            Token { tag: Tag::EndOfData, ..Default::default() },
            Token { tag: Tag::StartList, ..Default::default() },
            Token { tag: Tag::Call, ..Default::default() },
            Token { tag: Tag::EndList, ..Default::default() },
            Token { tag: Tag::EndOfSession, ..Default::default() },
        ];

        let mut splitter = Splitter::new();
        for token in tokens {
            splitter.enqueue(token);
        }
        assert!(splitter.poll().is_some_and(|result| result.is_err()));
        assert_eq!(splitter.poll(), Some(Ok(PackagedMethod::EndOfSession)));
        assert_eq!(splitter.poll(), None);
    }

    #[test]
    fn timer_empty() {
        let mut splitter = Timer::new(Properties { trans_timeout: Duration::from_secs(1000), ..Default::default() });
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(splitter.poll(), None);
    }

    #[test]
    fn timer_timed_out() {
        let mut splitter = Timer::new(Properties { trans_timeout: Duration::from_millis(0), ..Default::default() });
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(splitter.poll(), Some(Err(Error::TimedOut)));
        assert_eq!(splitter.poll(), Some(Err(Error::AbortedByHost))); // Error should be repeated.
    }

    #[test]
    fn timer_some() {
        let mut splitter = Timer::new(Properties { trans_timeout: Duration::from_secs(1000), ..Default::default() });
        splitter.enqueue_method(Ok(PackagedMethod::EndOfSession));
        assert_eq!(splitter.poll(), Some(Ok(PackagedMethod::EndOfSession)));
    }

    #[test]
    fn receiver_empty() {
        let mut receiver =
            PacketReceiver::new(Properties { trans_timeout: Duration::from_secs(0), ..Default::default() });
        std::thread::sleep(Duration::from_millis(50));
        assert!(receiver.poll().is_none());
    }

    #[test]
    fn receiver_some() {
        let mut receiver =
            PacketReceiver::new(Properties { trans_timeout: Duration::from_secs(1000), ..Default::default() });

        let packet = Packet {
            payload: vec![SubPacket { kind: SubPacketKind::Data, payload: vec![Tag::EndOfSession as u8].into() }]
                .into(),
            ..Default::default()
        };
        let (tx, _rx) = oneshot::channel();
        receiver.enqueue_promise(tx);
        assert!(receiver.poll().is_none());
        std::thread::sleep(Duration::from_millis(50));
        receiver.enqueue_packet(packet);
        assert!(receiver.poll().is_some_and(|result| result.1 == Ok(PackagedMethod::EndOfSession)));
    }

    #[test]
    fn receiver_timed_out() {
        let mut receiver =
            PacketReceiver::new(Properties { trans_timeout: Duration::from_secs(0), ..Default::default() });

        let packet = Packet {
            payload: vec![SubPacket { kind: SubPacketKind::Data, payload: vec![Tag::EndOfSession as u8].into() }]
                .into(),
            ..Default::default()
        };
        let (tx, _rx) = oneshot::channel();
        receiver.enqueue_promise(tx);
        std::thread::sleep(Duration::from_millis(50));
        assert!(receiver.poll().is_some_and(|result| result.1 == Err(Error::TimedOut)));
        receiver.enqueue_packet(packet);
        assert!(receiver.poll().is_none());
        let (tx, _rx) = oneshot::channel();
        receiver.enqueue_promise(tx);
        assert!(receiver.poll().is_some_and(|result| result.1 == Err(Error::AbortedByHost)));
    }
}
