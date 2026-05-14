//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::task::Poll::*;

use crate::messaging::packet::{Packet, SubPacket};
use crate::rpc::protocol::shared::pipe::{SinkPipe, SourcePipe};

pub type Input = Packet;
pub type Output = SubPacket;

pub fn flatten_packet(input: &mut dyn SourcePipe<Input>, output: &mut dyn SinkPipe<Output>) {
    while let Ready(Some(packet)) = input.pop() {
        for sub_packet in packet.payload.into_iter() {
            output.push(sub_packet);
        }
    }
}
