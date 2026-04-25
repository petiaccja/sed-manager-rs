use sed_packet::packet::Packet;

use crate::tper::TPer;

#[derive(Debug)]
pub struct Session {}

impl Session {
    pub fn dispatch(&mut self, _tper: &mut TPer, _packet: Packet) {
        todo!()
    }
}
