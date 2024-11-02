use std::collections::HashMap;

use super::route::{Route, RouteHandler};

/// The persistent state of the TCG device.
///
/// This would be written to disk and saved between power states
/// for an actual TCG device. Here, it only persists for the lifetime
/// of the fake device.
pub struct PersistentState {}

/// The part of the runtime state of the TCG device that contains the
/// routing table for com ID / protocol pairs.
///
/// This needs to be separate from the persistent state, which may be
/// modified by the handlers in the routing table, as the handlers are
/// part of the routing table, and that won't play well with the borrow
/// checker. It also won't work well if you bypass the borrow checker.
pub struct RoutingState {
    routes: HashMap<Route, Box<dyn RouteHandler>>,
}

impl RoutingState {
    pub fn new() -> RoutingState {
        RoutingState { routes: HashMap::new() }
    }

    pub fn get_route(&mut self, route: &Route) -> Option<&mut dyn RouteHandler> {
        Some(self.routes.get_mut(route)?.as_mut())
    }

    pub fn add_route<Handler: RouteHandler + 'static>(&mut self, route: Route, handler: Handler) -> Result<(), ()> {
        self.add_route_boxed(route, Box::new(handler))
    }

    pub fn add_route_boxed(&mut self, route: Route, handler: Box<dyn RouteHandler>) -> Result<(), ()> {
        if self.routes.contains_key(&route) {
            Err(())
        } else {
            self.routes.insert(route, handler);
            Ok(())
        }
    }

    pub fn remove_route(&mut self, route: Route) -> Result<(), ()> {
        if self.routes.contains_key(&route) {
            self.routes.remove(&route);
            Ok(())
        } else {
            Err(())
        }
    }
}
