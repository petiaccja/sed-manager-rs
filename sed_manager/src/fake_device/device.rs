use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::usize;

use crate::device::{Device, Error, Interface};
use crate::messaging::com_id::HANDLE_COM_ID_PROTOCOL;
use crate::messaging::packet::PACKETIZED_PROTOCOL;
use crate::rpc::Properties;

use super::com_id_session::ComIDSession;
use super::data::SSC;
use super::discovery::{get_discovery, write_discovery, BASE_COM_ID, NUM_COM_IDS};

const ROUTE_DISCOVERY: Route = Route { protocol: 0x01, com_id: 0x0001 };
const ROUTE_GET_COMID: Route = Route { protocol: 0x02, com_id: 0x0000 };
const ROUTE_TPER_RESET: Route = Route { protocol: 0x02, com_id: 0x0004 };

const CAPABILITIES: Properties = Properties {
    max_methods: usize::MAX,
    max_subpackets: usize::MAX,
    max_packets: usize::MAX,
    max_gross_packet_size: 65536,
    max_gross_compacket_size: 65536,
    max_gross_compacket_response_size: 65536,
    max_ind_token_size: 65480,
    max_agg_token_size: 65480,
    continued_tokens: false,
    seq_numbers: false,
    ack_nak: false,
    asynchronous: true,
    buffer_mgmt: false,
    max_retries: 3,
    trans_timeout: Duration::from_secs(10),
};

pub struct FakeDevice {
    capabilities: Properties,
    sessions: Mutex<HashMap<u16, ComIDSession>>,
    ssc: Arc<Mutex<SSC>>,
}

#[derive(PartialEq, Eq)]
struct Route {
    protocol: u8,
    com_id: u16,
}

impl FakeDevice {
    pub fn new() -> FakeDevice {
        let controller = Arc::new(Mutex::new(SSC::new()));
        let capabilities = CAPABILITIES;
        let mut sessions = HashMap::new();
        for i in 0..NUM_COM_IDS {
            let com_id = BASE_COM_ID + i;
            let session = ComIDSession::new(com_id, 0x0000, capabilities.clone(), controller.clone());
            sessions.insert(BASE_COM_ID + i, session);
        }
        FakeDevice { capabilities, ssc: controller, sessions: sessions.into() }
    }

    pub fn capabilities(&self) -> &Properties {
        &self.capabilities
    }
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
        let route = Route { protocol: security_protocol, com_id };

        if route == ROUTE_DISCOVERY {
            Ok(()) // Discovery on IF-SEND is simply ignored.
        } else if route == ROUTE_TPER_RESET {
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
        let route = Route { protocol: security_protocol, com_id };

        if route == ROUTE_DISCOVERY {
            write_discovery(&get_discovery(Properties::ASSUMED), len)
        } else if route == ROUTE_GET_COMID {
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
