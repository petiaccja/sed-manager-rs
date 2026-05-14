//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::frontend::Frontend;

pub mod digital_unit;

pub fn set_callbacks(frontend: Frontend) {
    digital_unit::set_callbacks(frontend.clone());
}
