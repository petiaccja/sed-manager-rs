use std::rc::Rc;

use crate::generated::AppWindow;

pub struct AppState {
    pub value: Rc<i32>,
    pub window: AppWindow,
}

impl AppState {
    pub fn new(window: AppWindow) -> Self {
        Self { value: 0.into(), window }
    }
}
