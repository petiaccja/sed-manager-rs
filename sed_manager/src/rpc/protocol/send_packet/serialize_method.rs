use core::task::Poll::*;

use crate::messaging::packet::{SubPacket, SubPacketKind};
use crate::messaging::token::SerializeTokens;
use crate::rpc::protocol::promise::Promise;
use crate::rpc::protocol::shared::pipe::{SinkPipe, SourcePipe};
use crate::rpc::protocol::tracing::trace_method;
use crate::rpc::{Error, PackagedMethod};
use crate::serialization::vec_without_len::VecWithoutLen;
use crate::serialization::SerializeBinary;

pub type Input = Promise<PackagedMethod, PackagedMethod, Error>;
pub type Output = Promise<SubPacket, PackagedMethod, Error>;

pub fn serialize_method(input: &mut dyn SourcePipe<Input>, output: &mut dyn SinkPipe<Output>) {
    while let Ready(Some(message)) = input.pop() {
        let message = message.try_map(|message| {
            trace_method(&message, "send");
            let tokens = message.to_tokens()?;
            let bytes = VecWithoutLen::from(tokens).to_bytes()?;
            let sub_packet = SubPacket { payload: bytes.into(), kind: SubPacketKind::Data };
            Ok(sub_packet)
        });
        if let Some(message) = message {
            output.push(message);
        };
    }
    if input.is_done() {
        output.close();
    }
}
