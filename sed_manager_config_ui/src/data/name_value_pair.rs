//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::NameValuePair;

impl NameValuePair {
    pub fn new(name: String, value: String) -> Self {
        Self { name: name.into(), value: value.into() }
    }
}
