use super::state::PersistentState;
use crate::device::Error as DeviceError;

// A TCG device uses the following routes:
//  - IF-SEND:
//      - (0x01, <valid>): session management & sessions
//      - (0x02, <valid>): HANDLE_COMID_REQUEST
//      - (0x02, 0x0004): TPER_RESET (opal)
//  - IF-RECV:
//      - 0x00: ????
//      - (0x01, 0x0001): discovery
//      - (0x01, <valid>): session management & sessions
//      - (0x02, ----): GET_COMID
//      - (0x02, <valid>): GET_COMID_RESPONSE

#[derive(PartialEq, Eq, Hash)]
pub struct Route {
    pub protocol: u8,
    pub com_id: u16,
}

pub struct RouteModifications {
    pub new_routes: Vec<(Route, Box<dyn RouteHandler>)>,
    pub deleted_routes: Vec<Route>,
}

pub trait RouteHandler {
    fn push_request(&mut self, state: &mut PersistentState, data: &[u8]) -> Result<RouteModifications, DeviceError>;
    fn pop_response(&mut self, state: &mut PersistentState, len: usize) -> Result<Vec<u8>, DeviceError>;
}

impl RouteModifications {
    pub fn none() -> RouteModifications {
        RouteModifications { new_routes: vec![], deleted_routes: vec![] }
    }
}
