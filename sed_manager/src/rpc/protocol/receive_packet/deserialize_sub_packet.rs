use core::cell::RefCell;
use core::task::Poll::*;
use std::rc::{Rc, Weak};

use crate::messaging::packet::{SubPacket, SubPacketKind};
use crate::messaging::token::Token;
use crate::rpc::protocol::shared::pipe::{SinkPipe, SourcePipe};
use crate::rpc::Error;

use crate::serialization::vec_without_len::VecWithoutLen;
use crate::serialization::DeserializeBinary;

pub type Input = SubPacket;
pub type Output = Result<Token, Error>;

pub fn deserialize_sub_packet(input: &mut dyn SourcePipe<Input>, output: &mut dyn SinkPipe<Output>) {
    while let Ready(Some(sub_packet)) = input.pop() {
        let tokens = match sub_packet.kind {
            SubPacketKind::Data => VecWithoutLen::<Token>::from_bytes(sub_packet.payload.into_vec()),
            _ => Ok(VecWithoutLen::new()),
        };
        match tokens {
            Ok(tokens) => tokens.into_iter().for_each(|token| output.push(Ok(token))),
            Err(error) => {
                output.push(Err(error.into()));
                output.close();
            }
        }
    }
}
