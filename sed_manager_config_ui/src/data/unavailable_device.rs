//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::UnavailableDevice;

impl UnavailableDevice {
    pub fn new(path: String, error_message: String) -> Self {
        Self { error_message: error_message.into(), path: path.into() }
    }

    pub fn empty() -> Self {
        Self::new(String::new(), String::new())
    }
}
