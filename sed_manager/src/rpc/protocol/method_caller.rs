use std::collections::VecDeque as Queue;
use std::ops::DerefMut;
use tokio::sync::Mutex;
use tokio::task::yield_now;

use super::traits::PacketLayer;
use crate::messaging::packet::{Packet, SubPacket, SubPacketKind, PACKET_HEADER_LEN, SUB_PACKET_HEADER_LEN};
use crate::messaging::token::{Tag, Token, TokenizeError};
use crate::messaging::value::Command;
use crate::rpc::error::Error;
use crate::rpc::method::{MethodCall, MethodResult};
use crate::rpc::properties::Properties;
use crate::serialization::vec_with_len::VecWithLen;
use crate::serialization::vec_without_len::VecWithoutLen;
use crate::serialization::{Deserialize, InputStream, ItemRead, OutputStream, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackagedMethod {
    Call(MethodCall),
    Result(MethodResult),
    EndOfSession,
}

pub struct MethodCaller {
    next_layer: Box<dyn PacketLayer>,
    properties: Properties,
    request_queue: Mutex<Queue<Vec<u8>>>,
    token_queue: Mutex<Queue<Token>>,
}

impl MethodCaller {
    pub fn new(next_layer: Box<dyn PacketLayer>, properties: Properties) -> Self {
        Self {
            next_layer,
            properties: properties,
            request_queue: Queue::new().into(),
            token_queue: Queue::new().into(),
        }
    }

    pub async fn send(&self, method: PackagedMethod) -> Result<(), Error> {
        {
            let tokens = tokenize(&method)?;
            let binary = serialize(tokens.as_slice())?;
            let validated = validate(binary, &self.properties)?;
            self.request_queue.lock().await.push_back(validated);
        };
        yield_now().await;
        {
            let request_queue = std::mem::replace(self.request_queue.lock().await.deref_mut(), Queue::new());
            let packets = bundle(request_queue, &self.properties);
            for packet in packets {
                self.next_layer.send(packet).await?;
            }
        };
        Ok(())
    }

    pub async fn recv(&self) -> Result<PackagedMethod, Error> {
        let mut token_queue = self.token_queue.lock().await;
        let mut method_parser = MethodParser::new();
        let result = 'outer: loop {
            while let Some(token) = token_queue.pop_front() {
                if let Some(method) = method_parser.feed(token)? {
                    break 'outer detokenize(method);
                }
            }
            let packet = self.next_layer.recv().await?;
            let tokens = deserialize(packet)?;
            for token in tokens {
                token_queue.push_back(token);
            }
        };
        result
    }

    pub async fn close(&self) {
        self.next_layer.close().await;
    }

    pub async fn abort(&self) {
        self.next_layer.abort().await;
    }
}

struct MethodParser {
    end_of_data: bool,
    list_depth: isize,
    tokens: Vec<Token>,
}

impl MethodParser {
    pub fn new() -> Self {
        Self { end_of_data: false, list_depth: 0, tokens: Vec::new() }
    }

    pub fn feed(&mut self, token: Token) -> Result<Option<Vec<Token>>, Error> {
        let tag = token.tag;
        self.tokens.push(token);
        if self.tokens.len() == 1 && tag == Tag::EndOfSession {
            Ok(Some(std::mem::replace(&mut self.tokens, Vec::new())))
        } else if !self.end_of_data {
            if tag == Tag::EndOfData {
                self.end_of_data = true;
            }
            Ok(None)
        } else {
            match tag {
                Tag::StartList => self.list_depth += 1,
                Tag::EndList => self.list_depth -= 1,
                _ => (),
            };
            if self.list_depth == 0 {
                self.end_of_data = false;
                Ok(Some(std::mem::replace(&mut self.tokens, Vec::new())))
            } else if self.list_depth < 0 {
                self.end_of_data = false;
                Err(Error::InvalidTokenStream)
            } else {
                Ok(None)
            }
        }
    }
}

fn tokenize(method: &PackagedMethod) -> Result<Vec<Token>, Error> {
    let mut stream = OutputStream::<Token>::new();
    match method.serialize(&mut stream) {
        Ok(_) => Ok(stream.take()),
        Err(err) => Err(Error::TokenizationFailed(err)),
    }
}

fn serialize(tokens: &[Token]) -> Result<Vec<u8>, Error> {
    let mut stream = OutputStream::<u8>::new();
    for token in tokens {
        if let Err(err) = token.serialize(&mut stream) {
            return Err(Error::SerializationFailed(err));
        };
    }
    Ok(stream.take())
}

fn validate(binary: Vec<u8>, properties: &Properties) -> Result<Vec<u8>, Error> {
    if binary.len() + PACKET_HEADER_LEN + SUB_PACKET_HEADER_LEN < properties.max_gross_packet_size {
        Ok(binary)
    } else {
        Err(Error::MethodTooLarge)
    }
}

fn bundle(mut request_queue: Queue<Vec<u8>>, _properties: &Properties) -> Vec<Packet> {
    let mut packets = Vec::new();
    while let Some(request) = request_queue.pop_front() {
        let sub_packet = SubPacket { kind: SubPacketKind::Data, payload: request.into() };
        let packet = Packet { payload: VecWithLen::from(vec![sub_packet]), ..Default::default() };
        packets.push(packet);
    }
    packets
}

fn deserialize(packet: Packet) -> Result<Queue<Token>, Error> {
    let mut tokens = Queue::<Token>::new();
    for sub_packet in packet.payload.into_vec() {
        if sub_packet.kind == SubPacketKind::Data {
            let mut stream = InputStream::from(sub_packet.payload.into_vec());
            let sub_tokens = match VecWithoutLen::<Token>::deserialize(&mut stream) {
                Ok(sub_tokens) => sub_tokens,
                Err(err) => return Err(Error::SerializationFailed(err)),
            };
            for token in sub_tokens.into_vec() {
                tokens.push_back(token);
            }
        }
    }
    Ok(tokens)
}

fn detokenize(tokens: Vec<Token>) -> Result<PackagedMethod, Error> {
    let mut stream = InputStream::from(tokens);
    match PackagedMethod::deserialize(&mut stream) {
        Ok(method) => Ok(method),
        Err(err) => Err(Error::TokenizationFailed(err)),
    }
}

impl Serialize<Token> for PackagedMethod {
    type Error = TokenizeError;
    fn serialize(&self, stream: &mut OutputStream<Token>) -> Result<(), Self::Error> {
        match self {
            PackagedMethod::Call(method_call) => method_call.serialize(stream),
            PackagedMethod::Result(method_result) => method_result.serialize(stream),
            PackagedMethod::EndOfSession => Command::EndOfSession.serialize(stream),
        }
    }
}

impl Deserialize<Token> for PackagedMethod {
    type Error = TokenizeError;
    fn deserialize(stream: &mut InputStream<Token>) -> Result<Self, Self::Error> {
        let Ok(first) = stream.peek_one() else {
            return Err(TokenizeError::EndOfStream);
        };
        match first.tag {
            Tag::Call => Ok(PackagedMethod::Call(MethodCall::deserialize(stream)?)),
            Tag::StartList => Ok(PackagedMethod::Result(MethodResult::deserialize(stream)?)),
            Tag::EndOfSession => {
                let _ = stream.read_one();
                Ok(PackagedMethod::EndOfSession)
            }
            _ => Err(TokenizeError::UnexpectedTag),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::rpc::{protocol::test::MockPacketLayer, MethodStatus};
    use crate::serialization::SeekAlways;
    use crate::serialization::{Error as SerializeError, SerializeBinary};

    use super::*;

    const PROPERTIES_SHORT: Properties = Properties { trans_timeout: Duration::from_millis(10), ..Properties::ASSUMED };

    #[test]
    fn serialize_packaged_method_call() -> Result<(), SerializeError> {
        let call = PackagedMethod::Call(MethodCall {
            invoking_id: 0xFFu64.into(),
            method_id: 0xEFu64.into(),
            args: vec![],
            status: MethodStatus::Fail,
        });
        let mut os = OutputStream::<Token>::new();
        call.serialize(&mut os)?;
        let stream_len = os.len();
        let mut is = InputStream::from(os.take());
        let copy = PackagedMethod::deserialize(&mut is)?;
        assert_eq!(call, copy);
        assert_eq!(is.pos(), stream_len);
        Ok(())
    }

    #[test]
    fn serialize_packaged_method_result() -> Result<(), SerializeError> {
        let call = PackagedMethod::Result(MethodResult { results: vec![], status: MethodStatus::Fail });
        let mut os = OutputStream::<Token>::new();
        call.serialize(&mut os)?;
        let stream_len = os.len();
        let mut is = InputStream::from(os.take());
        let copy = PackagedMethod::deserialize(&mut is)?;
        assert_eq!(call, copy);
        assert_eq!(is.pos(), stream_len);
        Ok(())
    }

    #[test]
    fn serialize_packaged_method_eos() -> Result<(), SerializeError> {
        let call = PackagedMethod::EndOfSession;
        let mut os = OutputStream::<Token>::new();
        call.serialize(&mut os)?;
        let stream_len = os.len();
        let mut is = InputStream::from(os.take());
        let copy = PackagedMethod::deserialize(&mut is)?;
        assert_eq!(call, copy);
        assert_eq!(is.pos(), stream_len);
        Ok(())
    }

    #[tokio::test]
    async fn send_normal() -> Result<(), Error> {
        let next_layer = Box::new(MockPacketLayer::new(Properties::ASSUMED));
        let method_caller = MethodCaller::new(next_layer.clone(), Properties::ASSUMED);
        method_caller.send(PackagedMethod::EndOfSession).await?;
        let enqueued = next_layer.take_enqueued().await;
        assert_eq!(enqueued.len(), 1);
        assert_eq!(enqueued[0].payload.len(), 1);
        assert_eq!(enqueued[0].payload[0].payload.len(), 1);
        method_caller.close().await;
        Ok(())
    }

    #[tokio::test]
    async fn recv_normal() -> Result<(), Error> {
        let next_layer = Box::new(MockPacketLayer::new(Properties::ASSUMED));
        let method_caller = MethodCaller::new(next_layer.clone(), Properties::ASSUMED);

        let payload = Token { tag: Tag::EndOfSession, ..Default::default() }.to_bytes().unwrap();
        let sub_packet = SubPacket { kind: SubPacketKind::Data, payload: payload.into() };
        let packet = Packet { payload: vec![sub_packet].into(), ..Default::default() };
        next_layer.add_dequeue(packet).await;
        let message = method_caller.recv().await?;

        assert_eq!(message, PackagedMethod::EndOfSession);
        method_caller.close().await;
        Ok(())
    }

    #[tokio::test]
    async fn recv_half_missing_timeout() {
        let next_layer = Box::new(MockPacketLayer::new(PROPERTIES_SHORT));
        let mut method_caller = MethodCaller::new(next_layer.clone(), PROPERTIES_SHORT);

        let payload = Token { tag: Tag::Call, ..Default::default() }.to_bytes().unwrap();
        let sub_packet = SubPacket { kind: SubPacketKind::Data, payload: payload.into() };
        let packet = Packet { payload: vec![sub_packet].into(), ..Default::default() };
        next_layer.add_dequeue(packet).await;
        let message = method_caller.recv().await;

        assert!(message.is_err_and(|err| err == Error::TimedOut));
        assert!(method_caller.token_queue.get_mut().is_empty());
        method_caller.close().await;
    }

    #[tokio::test]
    async fn recv_multiple_packets() -> Result<(), Error> {
        let next_layer = Box::new(MockPacketLayer::new(PROPERTIES_SHORT));
        let mut method_caller = MethodCaller::new(next_layer.clone(), PROPERTIES_SHORT);

        let call = PackagedMethod::Call(MethodCall {
            invoking_id: 0xFFu64.into(),
            method_id: 0xEFu64.into(),
            args: vec![],
            status: MethodStatus::Fail,
        });
        let tokens = tokenize(&call).unwrap();
        let tokens_first = &tokens.as_slice()[0..3];
        let tokens_second = &tokens.as_slice()[3..];
        let bytes_first = serialize(tokens_first).unwrap();
        let bytes_second = serialize(tokens_second).unwrap();
        let sub_packet_first = SubPacket { kind: SubPacketKind::Data, payload: Vec::from(bytes_first).into() };
        let sub_packet_second = SubPacket { kind: SubPacketKind::Data, payload: Vec::from(bytes_second).into() };
        let packet_first = Packet { payload: vec![sub_packet_first].into(), ..Default::default() };
        let packet_second = Packet { payload: vec![sub_packet_second].into(), ..Default::default() };
        next_layer.add_dequeue(packet_first).await;
        next_layer.add_dequeue(packet_second).await;
        let message = method_caller.recv().await?;

        assert_eq!(call, message);
        assert!(method_caller.token_queue.get_mut().is_empty());
        method_caller.close().await;
        Ok(())
    }
}
