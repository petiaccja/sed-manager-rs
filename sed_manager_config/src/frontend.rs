//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use slint::{ComponentHandle as _, Weak};

use crate::ui;

#[derive(Clone)]
pub struct Frontend {
    value: Weak<ui::AppWindow>,
}

impl Frontend {
    pub fn new(window: ui::AppWindow) -> Self {
        Self { value: window.as_weak() }
    }

    pub fn with<Output>(&self, f: impl FnOnce(ui::AppWindow) -> Output) -> Option<Output> {
        if let Some(value) = self.value.upgrade() {
            Some(f(value))
        } else {
            None
        }
    }
}
