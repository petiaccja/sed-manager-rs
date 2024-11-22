use std::io::Seek;

use tokio::sync::mpsc;

use crate::messaging::packet::{AckType, Packet, SubPacket, SubPacketKind, PACKET_HEADER_LEN, SUB_PACKET_HEADER_LEN};
use crate::messaging::token::{Tag, Token, TokenizeError};
use crate::rpc::error::Error;
use crate::rpc::method::{MethodCall, MethodResult};
use crate::rpc::pipeline::{
    connect, BufferedSend, Process, PullInput, PullOutput, PushInput, PushOutput, Receive, UnbufferedSend,
};
use crate::rpc::properties::Properties;
use crate::rpc::protocol::utils::into_rpc_result;
use crate::serialization::with_len::WithLen;
use crate::serialization::{Deserialize, InputStream, ItemRead, OutputStream, Serialize};

use super::utils::{aggregate_results, redirect_result, PullBridge, PushBridge};

pub enum PackagedMethod {
    Call(MethodCall),
    Result(MethodResult),
}

#[derive(PartialEq, Eq)]
enum ParseStage {
    /// Parsing the content of a method call or response.
    ///
    /// This section is terminated by receiving an end of data token.
    Data,
    /// Parsing the status list of a method call or response.
    ///
    /// The status list is a flat list.
    /// However, if somebody sends us garbage, we at least want to parse it correctly.
    /// The faulty list will later be discarded when the mathod call or response is detokenized.
    StatusList { depth: u32 },
}

pub struct MethodLayer {
    pub request: PullInput<PackagedMethod>,
    pub result_packet: PushInput<Packet>,
    pub request_packet: PullOutput<Packet>,
    pub result: PushOutput<PackagedMethod>,
    pub credit: PushOutput<u32>,
    properties: Properties,
}

struct Tokenize {
    pub input: PullInput<PackagedMethod>,
    pub output: PullOutput<Vec<Token>>,
}

struct SerializeTokens {
    pub input: PullInput<Vec<Token>>,
    pub output: PullOutput<Vec<u8>>,
}

struct Bundle {
    pub input: PullInput<Vec<u8>>,
    pub output: PullOutput<Packet>,
    properties: Properties,
}

struct ExtractData {
    pub input: PushInput<Packet>,
    pub output: PushOutput<Vec<u8>>,
}

struct DeserializeTokens {
    pub input: PushInput<Vec<u8>>,
    pub output: PushOutput<(u32, Vec<Token>)>,
}

struct SeparateMethods {
    pub input: PushInput<(u32, Vec<Token>)>,
    pub output: PushOutput<Vec<Token>>,
    pub credit: PushOutput<u32>,
    size: u32,
    tokens: Vec<Token>,
    parse_stage: ParseStage,
}

struct Detokenize {
    pub input: PushInput<Vec<Token>>,
    pub output: PushOutput<PackagedMethod>,
}

impl MethodLayer {
    pub fn new(properties: Properties) -> Self {
        Self {
            request: PullInput::new(),
            result_packet: PushInput::new(),
            request_packet: PullOutput::new(),
            result: PushOutput::new(),
            credit: PushOutput::new(),
            properties: properties,
        }
    }
}

impl Tokenize {
    pub fn new() -> Self {
        Self { input: PullInput::new(), output: PullOutput::new() }
    }
}

impl SerializeTokens {
    pub fn new() -> Self {
        Self { input: PullInput::new(), output: PullOutput::new() }
    }
}

impl Bundle {
    pub fn new(properties: Properties) -> Self {
        Self { input: PullInput::new(), output: PullOutput::new(), properties: properties }
    }
}

impl ExtractData {
    pub fn new() -> Self {
        Self { input: PushInput::new(), output: PushOutput::new() }
    }
}

impl DeserializeTokens {
    pub fn new() -> Self {
        Self { input: PushInput::new(), output: PushOutput::new() }
    }
}

impl SeparateMethods {
    pub fn new() -> Self {
        Self {
            input: PushInput::new(),
            output: PushOutput::new(),
            credit: PushOutput::new(),
            size: 0,
            tokens: Vec::new(),
            parse_stage: ParseStage::Data,
        }
    }

    fn push(&mut self, token: Token) -> Result<(), Error> {
        let tag = token.tag;
        self.tokens.push(token);

        if tag == Tag::EndOfData && self.parse_stage == ParseStage::Data {
            self.parse_stage = ParseStage::StatusList { depth: 0 };
            Ok(())
        } else if tag == Tag::StartList {
            if let ParseStage::StatusList { depth } = self.parse_stage {
                self.parse_stage = ParseStage::StatusList { depth: depth + 1 };
            };
            Ok(())
        } else if tag == Tag::EndList {
            if let ParseStage::StatusList { depth } = self.parse_stage {
                if depth == 0 {
                    Err(Error::InvalidTokenStream)
                } else {
                    self.parse_stage = ParseStage::StatusList { depth: depth - 1 };
                    self.commit();
                    Ok(())
                }
            } else {
                Ok(())
            }
        } else {
            self.commit();
            Ok(())
        }
    }

    fn commit(&mut self) {
        if let ParseStage::StatusList { depth: 0 } = self.parse_stage {
            let _ = self.output.send(std::mem::replace(&mut self.tokens, vec![]));
            let _ = self.credit.send(std::mem::replace(&mut self.size, 0));
            self.parse_stage = ParseStage::Data;
        }
    }
}

impl Detokenize {
    pub fn new() -> Self {
        Self { input: PushInput::new(), output: PushOutput::new() }
    }
}

impl Process for MethodLayer {
    type Output = ();
    type Error = Error;
    async fn update(&mut self) -> Result<Option<Self::Output>, Self::Error> {
        unreachable!()
    }
    async fn run(self) -> Result<Self::Output, Self::Error>
    where
        Self: Sized,
    {
        let cancel = tokio_util::sync::CancellationToken::new();

        let mut request_bridge = PullBridge { input: self.request, output: PullOutput::new(), cancel: cancel.clone() };
        let mut tokenize = Tokenize::new();
        let mut serialize = SerializeTokens::new();
        let mut bundle = Bundle::new(self.properties);
        connect(&mut request_bridge.output, &mut tokenize.input);
        connect(&mut tokenize.output, &mut serialize.input);
        connect(&mut serialize.output, &mut bundle.input);
        bundle.output = self.request_packet;

        let mut result_bridge =
            PushBridge { input: self.result_packet, output: PushOutput::new(), cancel: cancel.clone() };
        let mut data = ExtractData::new();
        let mut deserialize = DeserializeTokens::new();
        let mut separate_methods = SeparateMethods::new();
        let mut detokenize = Detokenize::new();
        connect(&mut result_bridge.output, &mut data.input);
        connect(&mut data.output, &mut deserialize.input);
        connect(&mut deserialize.output, &mut separate_methods.input);
        connect(&mut separate_methods.output, &mut detokenize.input);
        separate_methods.credit = self.credit;
        detokenize.output = self.result;

        let (result_tx, result_rx) = mpsc::unbounded_channel::<Result<(), Error>>();

        let _ = tokio::spawn(redirect_result(async { into_rpc_result(request_bridge.run().await) }, result_tx.clone()));
        let _ = tokio::spawn(redirect_result(tokenize.run(), result_tx.clone()));
        let _ = tokio::spawn(redirect_result(serialize.run(), result_tx.clone()));
        let _ = tokio::spawn(redirect_result(bundle.run(), result_tx.clone()));
        let _ = tokio::spawn(redirect_result(async { into_rpc_result(result_bridge.run().await) }, result_tx.clone()));
        let _ = tokio::spawn(redirect_result(data.run(), result_tx.clone()));
        let _ = tokio::spawn(redirect_result(deserialize.run(), result_tx.clone()));
        let _ = tokio::spawn(redirect_result(separate_methods.run(), result_tx.clone()));
        let _ = tokio::spawn(redirect_result(detokenize.run(), result_tx.clone()));

        drop(result_tx);
        aggregate_results(cancel, result_rx).await
    }
}

impl Process for Tokenize {
    type Output = ();
    type Error = Error;
    async fn update(&mut self) -> Result<Option<Self::Output>, Self::Error> {
        if let Some(input) = self.input.recv().await {
            let mut stream = OutputStream::<Token>::new();
            match input.serialize(&mut stream) {
                Ok(_) => {
                    let _ = self.output.send(stream.take()).await;
                    Ok(None)
                }
                Err(err) => Err(Error::TokenizationFailed(err)),
            }
        } else {
            Ok(Some(()))
        }
    }
}

impl Process for SerializeTokens {
    type Output = ();
    type Error = Error;
    async fn update(&mut self) -> Result<Option<Self::Output>, Self::Error> {
        if let Some(input) = self.input.recv().await {
            let mut stream = OutputStream::<u8>::new();
            for token in input {
                if let Err(err) = token.serialize(&mut stream) {
                    return Err(Error::SerializationFailed(err));
                };
            }
            let _ = self.output.send(stream.take()).await;
            Ok(None)
        } else {
            Ok(Some(()))
        }
    }
}

impl Process for Bundle {
    type Output = ();
    type Error = Error;
    async fn update(&mut self) -> Result<Option<Self::Output>, Self::Error> {
        if let Some(input) = self.input.recv().await {
            let projected_size = PACKET_HEADER_LEN + SUB_PACKET_HEADER_LEN + input.len();
            if projected_size <= self.properties.max_gross_packet_size {
                let sub_packet = SubPacket { kind: SubPacketKind::Data, payload: WithLen::new(input) };
                let packet = Packet {
                    host_session_number: 0,
                    tper_session_number: 0,
                    sequence_number: 0,
                    ack_type: AckType::None,
                    acknowledgement: 0,
                    payload: WithLen::new(vec![sub_packet]),
                };
                let _ = self.output.send(packet).await;
                Ok(None)
            } else {
                Err(Error::MethodTooLarge)
            }
        } else {
            Ok(Some(()))
        }
    }
}

impl Process for ExtractData {
    type Output = ();
    type Error = Error;
    async fn update(&mut self) -> Result<Option<Self::Output>, Self::Error> {
        if let Some(packet) = self.input.recv().await {
            for sub_packet in packet.payload.into_vec() {
                if sub_packet.kind == SubPacketKind::Data {
                    let _ = self.output.send(sub_packet.payload.into_vec());
                }
            }
            Ok(None)
        } else {
            Ok(Some(()))
        }
    }
}

impl Process for DeserializeTokens {
    type Output = ();
    type Error = Error;
    async fn update(&mut self) -> Result<Option<Self::Output>, Self::Error> {
        if let Some(input) = self.input.recv().await {
            let len = input.len();
            let mut stream = InputStream::<u8>::from(input);
            let mut tokens = Vec::new();
            while stream.stream_position().unwrap() != len as u64 {
                match Token::deserialize(&mut stream) {
                    Ok(token) => tokens.push(token),
                    Err(err) => return Err(Error::SerializationFailed(err)),
                }
            }
            let _ = self.output.send((len as u32, tokens));
            Ok(None)
        } else {
            Ok(Some(()))
        }
    }
}

impl Process for SeparateMethods {
    type Output = ();
    type Error = Error;
    async fn update(&mut self) -> Result<Option<Self::Output>, Self::Error> {
        if let Some((size, tokens)) = self.input.recv().await {
            for token in tokens {
                if let Err(err) = self.push(token) {
                    return Err(err);
                }
            }
            self.size += size;
            Ok(None)
        } else {
            Ok(Some(()))
        }
    }
}

impl Process for Detokenize {
    type Output = ();
    type Error = Error;
    async fn update(&mut self) -> Result<Option<Self::Output>, Self::Error> {
        if let Some(tokens) = self.input.recv().await {
            let mut stream = InputStream::<Token>::from(tokens);
            if let Ok(first) = stream.peek_one() {
                if first.tag == Tag::Call {
                    match MethodCall::deserialize(&mut stream) {
                        Ok(method_call) => {
                            let _ = self.output.send(PackagedMethod::Call(method_call));
                            Ok(None)
                        }
                        Err(err) => Err(Error::TokenizationFailed(err)),
                    }
                } else {
                    match MethodResult::deserialize(&mut stream) {
                        Ok(method_result) => {
                            let _ = self.output.send(PackagedMethod::Result(method_result));
                            Ok(None)
                        }
                        Err(err) => Err(Error::TokenizationFailed(err)),
                    }
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(Some(()))
        }
    }
}

impl Serialize<PackagedMethod, Token> for PackagedMethod {
    type Error = TokenizeError;
    fn serialize(&self, stream: &mut OutputStream<Token>) -> Result<(), Self::Error> {
        match self {
            PackagedMethod::Call(method_call) => method_call.serialize(stream),
            PackagedMethod::Result(method_result) => method_result.serialize(stream),
        }
    }
}

impl Deserialize<PackagedMethod, Token> for PackagedMethod {
    type Error = TokenizeError;
    fn deserialize(stream: &mut crate::serialization::InputStream<Token>) -> Result<PackagedMethod, Self::Error> {
        let Ok(first) = stream.peek_one() else {
            return Err(TokenizeError::EndOfStream);
        };
        match first.tag {
            Tag::Call => Ok(PackagedMethod::Call(MethodCall::deserialize(stream)?)),
            Tag::StartList => Ok(PackagedMethod::Result(MethodResult::deserialize(stream)?)),
            _ => Err(TokenizeError::UnexpectedTag),
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::task::yield_now;

    use crate::messaging::value::Value;
    use crate::rpc::test_utils::{create_sink_push, create_source_push, run_async_test, try_collect_push};
    use crate::rpc::method::MethodStatus;
    use crate::rpc::pipeline::spawn;

    use super::*;

    fn example_method_call() -> MethodCall {
        MethodCall {
            invoking_id: 1_u64.into(),
            method_id: 2_u64.into(),
            args: vec![Value::from(vec![Value::from(6_u16)])],
        }
    }

    fn example_method_result() -> MethodResult {
        MethodResult { results: vec![Value::from(6_u16)], status: MethodStatus::NotAuthorized }
    }

    #[test]
    fn separate_methods() -> Result<(), Error> {
        run_async_test(async {
            let mut stream = OutputStream::<Token>::new();
            example_method_call().serialize(&mut stream).unwrap();
            example_method_result().serialize(&mut stream).unwrap();

            let mut process = SeparateMethods::new();
            let mut source = create_source_push(&mut process.input);
            let mut sink = create_sink_push(&mut process.output);

            source.send((100, stream.take())).unwrap();
            let process = spawn(process);
            yield_now().await;
            let mut result = Vec::new();
            try_collect_push(&mut result, &mut sink).await;

            assert_eq!(result.len(), 2);
            let mut is_call = InputStream::from(std::mem::replace(&mut result[0], vec![]));
            let mut is_result = InputStream::from(std::mem::replace(&mut result[1], vec![]));
            assert_eq!(MethodCall::deserialize(&mut is_call).unwrap(), example_method_call());
            assert_eq!(MethodResult::deserialize(&mut is_result).unwrap(), example_method_result());

            drop(source);

            process.await
        })
    }
}
