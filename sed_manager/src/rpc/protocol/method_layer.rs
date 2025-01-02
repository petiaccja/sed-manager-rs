use std::collections::VecDeque as Queue;
use std::ops::DerefMut;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use tokio::sync::Mutex;
use tokio::task::yield_now;

use super::buffer::Buffer;
use super::packet_layer::PacketLayer;
use crate::messaging::packet::{Packet, SubPacket, SubPacketKind, PACKET_HEADER_LEN, SUB_PACKET_HEADER_LEN};
use crate::messaging::token::{Tag, Token, TokenizeError};
use crate::messaging::value::Command;
use crate::rpc::error::Error;
use crate::rpc::method::{MethodCall, MethodResult};
use crate::rpc::properties::Properties;
use crate::serialization::vec_with_len::VecWithLen;
use crate::serialization::vec_without_len::VecWithoutLen;
use crate::serialization::{Deserialize, InputStream, ItemRead, OutputStream, Serialize};

const BUFFER_CAPACITY: u32 = 1048576;

pub enum PackagedMethod {
    Call(MethodCall),
    Result(MethodResult),
    EndOfSession,
}

pub struct MethodLayer {
    next_layer: Box<dyn PacketLayer>,
    properties: Properties,
    request_queue: Mutex<Queue<Vec<u8>>>,
    token_queue: Mutex<Queue<Token>>,
    buffer: Buffer,
    outstanding_credit: AtomicU32,
}

impl MethodLayer {
    pub fn new(next_layer: Box<dyn PacketLayer>, initial_credit_sent: u32, properties: Properties) -> Self {
        let capacity = std::cmp::max(initial_credit_sent, BUFFER_CAPACITY);
        let buffer = Buffer::new(capacity);
        Self {
            next_layer,
            properties: properties,
            request_queue: Queue::new().into(),
            token_queue: Queue::new().into(),
            buffer,
            outstanding_credit: (capacity - initial_credit_sent).into(),
        }
    }

    pub async fn send(&self, method: PackagedMethod) -> Result<(), Error> {
        {
            let tokens = tokenize(method)?;
            let binary = serialize(tokens)?;
            let validated = validate(binary, &self.properties)?;
            self.request_queue.lock().await.push_back(validated);
        };
        yield_now().await;
        {
            if let Some(packet) = self.commit_outstanding_credit() {
                self.next_layer.send(packet).await?;
            }
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
        let mut allocated: u32 = 0;
        let result = 'outer: loop {
            while let Some(token) = token_queue.pop_front() {
                if let Some(method) = method_parser.feed(token)? {
                    break 'outer detokenize(method);
                }
            }
            let packet = self.next_layer.recv().await?;
            let credit = packet.credit();
            if self.buffer.allocate(credit) {
                allocated += credit;
            } else {
                break 'outer Err(Error::OutOfCreditRemote);
            };
            let tokens = deserialize(packet)?;
            for token in tokens {
                token_queue.push_back(token);
            }
        };
        self.buffer.deallocate(allocated);
        self.outstanding_credit.fetch_add(allocated, Ordering::Relaxed);
        result
    }

    pub async fn close(&self) {
        self.next_layer.close().await;
    }

    fn commit_outstanding_credit(&self) -> Option<Packet> {
        let credit = self.outstanding_credit.swap(0, Ordering::Relaxed);
        if credit != 0 {
            let payload = Vec::from(credit.to_be_bytes()).into();
            let sub_packet = SubPacket { kind: SubPacketKind::CreditControl, payload };
            Some(Packet { payload: vec![sub_packet].into(), ..Default::default() })
        } else {
            None
        }
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
        if !self.end_of_data {
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

fn tokenize(method: PackagedMethod) -> Result<Vec<Token>, Error> {
    let mut stream = OutputStream::<Token>::new();
    match method.serialize(&mut stream) {
        Ok(_) => Ok(stream.take()),
        Err(err) => Err(Error::TokenizationFailed(err)),
    }
}

fn serialize(tokens: Vec<Token>) -> Result<Vec<u8>, Error> {
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
            Tag::EndOfSession => Ok(PackagedMethod::EndOfSession),
            _ => Err(TokenizeError::UnexpectedTag),
        }
    }
}
