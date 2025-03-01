use core::task::Poll::*;

use crate::messaging::packet::{Packet, SubPacket, PACKET_HEADER_LEN, SUB_PACKET_HEADER_LEN};
use crate::rpc::protocol::promise::Promise;
use crate::rpc::protocol::shared::pipe::{SinkPipe, SourcePipe};
use crate::rpc::{Error, ErrorEvent, ErrorEventExt, PackagedMethod, Properties};

pub type Input = Promise<SubPacket, PackagedMethod, Error>;
pub type Output = Promise<Packet, PackagedMethod, Error>;

pub fn assemble_packet(input: &mut dyn SourcePipe<Input>, output: &mut dyn SinkPipe<Output>, properties: &Properties) {
    while let Ready(Some(message)) = input.pop() {
        let message = message.try_map(|message| {
            let size = message.payload.len() + PACKET_HEADER_LEN + SUB_PACKET_HEADER_LEN;
            if size > properties.max_gross_packet_size {
                return Err(ErrorEvent::MethodTooLarge.as_error());
            }
            let packet = Packet { payload: vec![message].into(), ..Default::default() };
            Ok(packet)
        });
        if let Some(message) = message {
            output.push(message);
        };
    }
    if input.is_done() {
        output.close();
    }
}
