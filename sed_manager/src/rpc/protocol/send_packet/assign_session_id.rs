use core::task::Poll::*;

use crate::messaging::packet::Packet;
use crate::rpc::protocol::promise::Promise;
use crate::rpc::protocol::shared::pipe::{SinkPipe, SourcePipe};
use crate::rpc::{Error, PackagedMethod, SessionIdentifier};

pub type Input = Promise<Packet, PackagedMethod, Error>;
pub type Output = Promise<Packet, PackagedMethod, Error>;

pub fn assign_session_id(input: &mut dyn SourcePipe<Input>, output: &mut dyn SinkPipe<Output>, id: &SessionIdentifier) {
    while let Ready(Some(message)) = input.pop() {
        let message =
            message.map(|packet| Packet { host_session_number: id.hsn, tper_session_number: id.tsn, ..packet });
        output.push(message);
    }
    if input.is_done() {
        output.close();
    }
}
