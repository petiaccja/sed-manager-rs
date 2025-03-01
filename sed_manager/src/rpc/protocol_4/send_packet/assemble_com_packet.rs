use core::task::Poll::*;

use crate::messaging::packet::{ComPacket, Packet};
use crate::rpc::protocol_4::promise::Promise;
use crate::rpc::protocol_4::shared::pipe::{SinkPipe, SourcePipe};
use crate::rpc::{Error, PackagedMethod, SessionIdentifier};

pub type Input = Promise<Packet, PackagedMethod, Error>;
pub type OutputPromise = Promise<SessionIdentifier, PackagedMethod, Error>;
pub type Output = (ComPacket, Vec<OutputPromise>);

pub struct AssembleComPacket {
    com_id: u16,
    com_id_ext: u16,
}

impl AssembleComPacket {
    pub fn new(com_id: u16, com_id_ext: u16) -> Self {
        Self { com_id, com_id_ext }
    }

    pub fn update(&self, input: &mut dyn SourcePipe<Input>, output: &mut dyn SinkPipe<Output>) {
        while let Ready(Some(message)) = input.pop() {
            let (packet, senders) = message.detach();
            let id = SessionIdentifier { hsn: packet.host_session_number, tsn: packet.tper_session_number };
            let com_packet = ComPacket {
                com_id: self.com_id,
                com_id_ext: self.com_id_ext,
                payload: vec![packet].into(),
                ..Default::default()
            };
            let message = Promise::new(id, senders);
            output.push((com_packet, vec![message]));
        }
        if input.is_done() {
            output.close();
        }
    }
}
