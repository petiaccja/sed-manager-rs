//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::spec::opal::admin::*;
use crate::spec::{self, objects::CPIN};
use crate::virtual_device::PSID_PASSWORD;
use crate::virtual_device::{MSID_PASSWORD, data::object_table::CPINTable};

use super::ADMIN_IDX;

pub fn preconfig_c_pin() -> CPINTable {
    let mut items = vec![
        CPIN { uid: c_pin::SID, pin: MSID_PASSWORD.into(), ..Default::default() },
        CPIN { uid: c_pin::MSID, pin: MSID_PASSWORD.into(), ..Default::default() },
        CPIN { uid: spec::psid::admin::c_pin::PSID, pin: PSID_PASSWORD.into(), ..Default::default() },
    ];

    for admin_idx in ADMIN_IDX {
        items.push(CPIN {
            uid: c_pin::ADMIN.nth(admin_idx).unwrap(),
            pin: "8965823nz987gt346".into(),
            ..Default::default()
        });
    }

    items.into_iter().collect()
}
