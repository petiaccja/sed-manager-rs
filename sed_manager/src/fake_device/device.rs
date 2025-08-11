//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::time::Duration;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

use crate::device::{Device, Error, Interface};
use crate::fake_device::data::opal_v2;
use crate::fake_device::tper::TPer;
use crate::messaging::com_id::HANDLE_COM_ID_PROTOCOL;
use crate::messaging::packet::PACKETIZED_PROTOCOL;
use crate::rpc::{Properties, SessionIdentifier};
use crate::spec::column_types::SPRef;

use super::com_id_session::ComIDSession;
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
    state: Arc<Mutex<DeviceState>>,
}

struct DeviceState {
    tper: TPer,
    com_id_session: ComIDSession,
}

#[derive(PartialEq, Eq)]
struct Route {
    pub protocol: u8,
    pub com_id: u16,
}

impl FakeDevice {
    pub fn new() -> FakeDevice {
        assert_eq!(
            NUM_COM_IDS, 1,
            "only a single ComID is supported due to lack of ComID multiplexing in firmware state"
        );
        let tper = opal_v2::new_controller();
        let state =
            DeviceState { tper: TPer::new(tper, CAPABILITIES), com_id_session: ComIDSession::new(BASE_COM_ID, 0x0000) };
        FakeDevice { state: Arc::new(Mutex::new(state)) }
    }

    pub fn capabilities(&self) -> Properties {
        let state = self.state.lock().unwrap();
        state.tper.protocol_stack.capabilities.clone()
    }

    pub fn active_sessions(&self) -> Vec<(SessionIdentifier, SPRef)> {
        let state = self.state.lock().unwrap();
        state
            .tper
            .protocol_stack
            .list_sessions()
            .filter_map(|session_id| {
                state.tper.protocol_stack.get_session(*session_id).map(|session| (*session_id, session.sp))
            })
            .collect()
    }

    pub fn with_tper<T>(&self, f: impl FnOnce(&TPer) -> T) -> T {
        let state = self.state.lock().unwrap();
        f(&state.tper)
    }

    pub fn with_tper_mut<T>(&self, f: impl FnOnce(&mut TPer) -> T) -> T {
        let mut state = self.state.lock().unwrap();
        f(&mut state.tper)
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
        let mut state = self.state.lock().unwrap();
        let DeviceState { tper: firmware, com_id_session: session } = state.deref_mut();

        if route == ROUTE_DISCOVERY {
            Ok(()) // Discovery on IF-SEND is simply ignored.
        } else if route == ROUTE_TPER_RESET {
            unimplemented!("TPer reset is not implemented for the fake device")
        } else if session.com_id() == com_id {
            match security_protocol {
                HANDLE_COM_ID_PROTOCOL => session.on_security_send_com(firmware, data),
                PACKETIZED_PROTOCOL => session.on_security_send_packet(firmware, data),
                _ => Err(Error::InvalidProtocolOrComID),
            }
        } else {
            Err(Error::InvalidProtocolOrComID)
        }
    }

    fn security_recv(&self, security_protocol: u8, protocol_specific: [u8; 2], len: usize) -> Result<Vec<u8>, Error> {
        let com_id = u16::from_be_bytes(protocol_specific);
        let route = Route { protocol: security_protocol, com_id };
        let mut state = self.state.lock().unwrap();

        if route == ROUTE_DISCOVERY {
            let discovery = get_discovery(&&state.tper.protocol_stack.capabilities, &state.tper.ssc);
            write_discovery(&discovery, len)
        } else if route == ROUTE_GET_COMID {
            unimplemented!("dynamic com ID management is not implemented for the fake device")
        } else if state.com_id_session.com_id() == com_id {
            match security_protocol {
                HANDLE_COM_ID_PROTOCOL => state.com_id_session.on_security_recv_com(len),
                PACKETIZED_PROTOCOL => state.com_id_session.on_security_recv_packet(len),
                _ => Err(Error::InvalidProtocolOrComID),
            }
        } else {
            Err(Error::InvalidProtocolOrComID)
        }
    }
}
