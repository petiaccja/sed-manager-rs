//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::time::Duration;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::device::{Device, Error, Interface};
use crate::messaging::com_id::HANDLE_COM_ID_PROTOCOL;
use crate::messaging::packet::PACKETIZED_PROTOCOL;
use crate::rpc::{Properties, SessionIdentifier};
use crate::spec::column_types::SPRef;

use super::com_id_session::ComIDSession;
use super::data::OpalV2Controller;
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
    def_trans_timeout: Duration::from_secs(10),
};

pub struct FakeDevice {
    capabilities: Properties,
    sessions: Mutex<HashMap<u16, ComIDSession>>,
    controller: Arc<Mutex<OpalV2Controller>>,
}

#[derive(PartialEq, Eq)]
struct Route {
    protocol: u8,
    com_id: u16,
}

impl FakeDevice {
    pub fn new() -> FakeDevice {
        let controller = Arc::new(Mutex::new(OpalV2Controller::new()));
        let capabilities = CAPABILITIES;
        let mut sessions = HashMap::new();
        for i in 0..NUM_COM_IDS {
            let com_id = BASE_COM_ID + i;
            let session = ComIDSession::new(com_id, 0x0000, capabilities.clone(), controller.clone());
            sessions.insert(BASE_COM_ID + i, session);
        }
        FakeDevice { capabilities, controller, sessions: sessions.into() }
    }

    pub fn capabilities(&self) -> &Properties {
        &self.capabilities
    }

    pub fn active_sessions(&self) -> Vec<(SessionIdentifier, SPRef)> {
        let sessions = self.sessions.lock().unwrap();
        sessions.iter().map(|session| session.1.active_sessions()).flatten().collect()
    }

    pub fn controller(&self) -> Arc<Mutex<OpalV2Controller>> {
        self.controller.clone()
    }
}

impl Device for FakeDevice {
    fn path(&self) -> Option<String> {
        None
    }

    fn interface(&self) -> Interface {
        Interface::Other
    }

    fn model_number(&self) -> String {
        String::from("Virtual Test Device")
    }

    fn serial_number(&self) -> String {
        String::from("SN123456")
    }

    fn firmware_revision(&self) -> String {
        String::from("FW1234")
    }

    fn is_security_supported(&self) -> bool {
        true
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
            let controller = self.controller.lock().unwrap();
            let discovery = get_discovery(&self.capabilities, &controller);
            write_discovery(&discovery, len)
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
