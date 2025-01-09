use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::device::{Device, Error, Interface};
use crate::messaging::com_id::HANDLE_COM_ID_PROTOCOL;
use crate::messaging::packet::PACKETIZED_PROTOCOL;
use crate::rpc::ASSUMED_PROPERTIES;

use super::controller::Controller;
use super::discovery::{get_discovery, write_discovery, BASE_COM_ID, NUM_COM_IDS};
use super::session::Session;

const ROUTE_DISCOVERY: (u8, u16) = (0x01, 0x0001);
const ROUTE_GET_COMID: (u8, u16) = (0x02, 0x0000);
const ROUTE_TPER_RESET: (u8, u16) = (0x02, 0x0004);

pub struct FakeDevice {
    sessions: Mutex<HashMap<u16, Session>>,
    controller: Arc<Mutex<Controller>>,
}

impl Device for FakeDevice {
    fn interface(&self) -> Interface {
        Interface::Other
    }

    fn model_number(&self) -> Result<String, Error> {
        Ok(String::from("FAKE-DEV-OPAL-2"))
    }

    fn serial_number(&self) -> Result<String, Error> {
        Ok(String::from("0123456789"))
    }

    fn firmware_revision(&self) -> Result<String, Error> {
        Ok(String::from("FW1.0"))
    }

    fn security_send(&self, security_protocol: u8, protocol_specific: [u8; 2], data: &[u8]) -> Result<(), Error> {
        let com_id = u16::from_be_bytes(protocol_specific);

        if (security_protocol, com_id) == ROUTE_DISCOVERY {
            Ok(()) // Discovery on IF-SEND is simply ignored.
        } else if (security_protocol, com_id) == ROUTE_TPER_RESET {
            unimplemented!("TPer reset is not implemented for the fake device")
        } else if let Some(session) = self.sessions.lock().unwrap().get_mut(&com_id) {
            match security_protocol {
                HANDLE_COM_ID_PROTOCOL => session.on_security_send_com(data),
                PACKETIZED_PROTOCOL => session.on_security_send_packet(data),
                _ => Err(Error::InvalidProtocolOrComID),
            }
        } else {
            Err(Error::InvalidProtocolOrComID)
        }
    }

    fn security_recv(&self, security_protocol: u8, protocol_specific: [u8; 2], len: usize) -> Result<Vec<u8>, Error> {
        let com_id = u16::from_be_bytes(protocol_specific);

        if (security_protocol, com_id) == ROUTE_DISCOVERY {
            write_discovery(&get_discovery(ASSUMED_PROPERTIES), len)
        } else if (security_protocol, com_id) == ROUTE_GET_COMID {
            unimplemented!("dynamic com ID management is not implemented for the fake device")
        } else if let Some(session) = self.sessions.lock().unwrap().get_mut(&com_id) {
            match security_protocol {
                HANDLE_COM_ID_PROTOCOL => session.on_security_recv_com(len),
                PACKETIZED_PROTOCOL => session.on_security_recv_packet(len),
                _ => Err(Error::InvalidProtocolOrComID),
            }
        } else {
            Err(Error::InvalidProtocolOrComID)
        }
    }
}

impl FakeDevice {
    pub fn new() -> FakeDevice {
        let controller = Arc::new(Mutex::new(Controller {}));
        let mut sessions = HashMap::new();
        for i in 0..NUM_COM_IDS {
            let com_id = BASE_COM_ID + i;
            let session = Session::new(com_id, 0x0000, controller.clone());
            sessions.insert(BASE_COM_ID + i, session);
        }
        FakeDevice { controller, sessions: sessions.into() }
    }
}
