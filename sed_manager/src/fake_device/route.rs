use super::state::PersistentState;
use crate::device::Error as DeviceError;

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
