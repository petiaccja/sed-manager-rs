use std::collections::HashMap;
use std::sync::Mutex;

use sed_device::{Device, Error, Interface};
use sed_packet::com_id::HandleComIdRequest;
use sed_packet::packet::ComPacket;
use sorbit::ser_de::{FromBytes, ToBytes as _};

use crate::com_id::ComId;
use crate::com_session::ComSession;
use crate::packet_session::PacketSession;
use crate::tper::TPer;

pub struct VirtualDevice {
    tper: Mutex<TPer>,
    com_sessions: Mutex<HashMap<ComId, ComSession>>,
    packet_sessions: Mutex<HashMap<ComId, PacketSession>>,
}

impl Device for VirtualDevice {
    fn path(&self) -> Option<String> {
        None
    }

    fn interface(&self) -> Interface {
        Interface::Other
    }

    fn model_number(&self) -> String {
        "Virtual Device".into()
    }

    fn serial_number(&self) -> String {
        "SERIAL01".into()
    }

    fn firmware_revision(&self) -> String {
        "FWREV01".into()
    }

    fn is_security_supported(&self) -> bool {
        true
    }

    fn security_send(&self, security_protocol: u8, protocol_specific: [u8; 2], data: &[u8]) -> Result<(), Error> {
        let mut tper = self.tper.lock().expect("the virtual device panicked in another thread");
        let mut com_sessions = self.com_sessions.lock().expect("the virtual device panicked in another thread");
        let mut packet_sessions = self.packet_sessions.lock().expect("the virtual device panicked in another thread");

        let com_id = u16::from_be_bytes(protocol_specific);
        match (security_protocol, com_id) {
            // Discovery: ignore
            (0x01, 0x0001) => Ok(()),
            // Communication layer
            (0x02, com_id) if let Some(session) = com_sessions.get_mut(&ComId(com_id)) => {
                match HandleComIdRequest::from_bytes(data) {
                    Ok(request) => {
                        let command = session.push(&*packet_sessions, request);
                        if let Some(command) = command {
                            com_sessions.remove(&command.0);
                            packet_sessions.remove(&command.0);
                        }
                        Ok(())
                    }
                    Err(_) => Err(Error::InvalidArgument),
                }
            }
            // Packet layer
            (0x01, com_id) if let Some(session) = packet_sessions.get_mut(&ComId(com_id)) => {
                match ComPacket::from_bytes(data) {
                    Ok(com_packet) => {
                        session.push(&mut tper, com_packet);
                        Ok(())
                    }
                    Err(_) => Err(Error::InvalidArgument),
                }
            }
            (_, _) => Err(Error::InvalidProtocolOrComID),
        }
    }

    fn security_recv(&self, security_protocol: u8, protocol_specific: [u8; 2], len: usize) -> Result<Vec<u8>, Error> {
        let mut tper = self.tper.lock().expect("the virtual device panicked in another thread");
        let mut com_sessions = self.com_sessions.lock().expect("the virtual device panicked in another thread");
        let mut packet_sessions = self.packet_sessions.lock().expect("the virtual device panicked in another thread");

        let com_id = u16::from_be_bytes(protocol_specific);
        match (security_protocol, com_id) {
            // Discovery
            (0x01, 0x0001) => {
                let mut message = tper.pop_discovery();
                // In case `len` is too small, we must return the truncated buffer.
                message.resize(len, 0);
                Ok(message)
            }
            // Get ComID
            (0x02, 0x0000) => Err(Error::NotSupported),
            // Communication layer
            (0x02, com_id) if let Some(session) = com_sessions.get_mut(&ComId(com_id)) => {
                let response = session.pop();
                let mut message = response.to_bytes().expect("serializing ComID response failed");
                message.resize(len, 0);
                Ok(message)
            }
            // Packet layer
            (0x01, com_id) if let Some(session) = packet_sessions.get_mut(&ComId(com_id)) => {
                session.pop(len).map(|com_packet| {
                    let mut bytes = com_packet.to_bytes().expect("serializing ComPacket failed");
                    bytes.resize(len, 0);
                    bytes
                })
            }
            (_, _) => Err(Error::InvalidProtocolOrComID),
        }
    }
}
