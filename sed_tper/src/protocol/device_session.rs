use std::collections::VecDeque;

use sed_device::Device;

use crate::protocol::{
    messages::{SendComIdRequest, SendPacket},
    protocol::{Context, Topic},
};

pub struct DeviceSession {
    device: Box<dyn Device>,
    packet_queue: VecDeque<SendPacket>,
    com_id_queue: VecDeque<SendComIdRequest>
}

enum PacketState {
    Sending,
    Receiving,
}

impl DeviceSession {
    fn topic(&self) -> Topic {
        Topic::DeviceLayer
    }

    fn on_send_packet(&mut self, context: &mut Context, message: SendPacket) {

    }

    fn on_send_com_id_request(&mut self, context: &mut Context, message: SendComIdRequest) {}
}
