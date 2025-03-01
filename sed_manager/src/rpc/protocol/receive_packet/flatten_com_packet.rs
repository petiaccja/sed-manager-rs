use core::task::Poll::*;

use crate::messaging::packet::{ComPacket, Packet};
use crate::rpc::protocol::shared::pipe::{SinkPipe, SourcePipe};

pub type Input = ComPacket;
pub type Output = Packet;

pub fn flatten_com_packet(input: &mut dyn SourcePipe<Input>, output: &mut dyn SinkPipe<Output>) {
    while let Ready(Some(com_packet)) = input.pop() {
        for packet in com_packet.payload.into_iter() {
            output.push(packet);
        }
    }
}
