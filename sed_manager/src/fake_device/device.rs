use std::cell::RefCell;

use crate::device::{Device, Error, Interface};

use super::discovery::DiscoveryHandler;
use super::route::Route;
use super::state::{PersistentState, RoutingState};

pub struct FakeDevice {
    routing_state: RefCell<RoutingState>,
    persistent_state: RefCell<PersistentState>,
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
        let route = Route { protocol: security_protocol, com_id: com_id };
        let mut routing_state = self.routing_state.borrow_mut();
        let Some(handler) = routing_state.get_route(&route) else {
            return Err(Error::InvalidProtocolOrComID);
        };
        let mut persistent_state = self.persistent_state.borrow_mut();
        let route_modifications = handler.push_request(&mut persistent_state, data)?;
        for (route, handler) in route_modifications.new_routes {
            routing_state.add_route_boxed(route, handler).expect("push_request should not return invalid routes");
        }
        for route in route_modifications.deleted_routes {
            routing_state.remove_route(route).expect("push_request should not return invalid routes");
        }
        Ok(())
    }

    fn security_recv(&self, security_protocol: u8, protocol_specific: [u8; 2], len: usize) -> Result<Vec<u8>, Error> {
        let com_id = u16::from_be_bytes(protocol_specific);
        let route = Route { protocol: security_protocol, com_id: com_id };
        let mut routing_state = self.routing_state.borrow_mut();
        let Some(handler) = routing_state.get_route(&route) else {
            return Err(Error::InvalidProtocolOrComID);
        };
        let mut persistent_state = self.persistent_state.borrow_mut();
        handler.pop_response(&mut persistent_state, len)
    }
}

impl FakeDevice {
    pub fn new() -> FakeDevice {
        let routing_state = FakeDevice::default_routes();
        let persistent_state = PersistentState {};
        FakeDevice { routing_state: routing_state.into(), persistent_state: persistent_state.into() }
    }

    fn default_routes() -> RoutingState {
        let mut state = RoutingState::new();
        state.add_route(Route { protocol: 0x01, com_id: 0x0001 }, DiscoveryHandler {}).unwrap();
        state
    }
}
