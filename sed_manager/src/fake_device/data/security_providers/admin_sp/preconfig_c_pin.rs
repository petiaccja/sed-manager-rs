use crate::fake_device::PSID_PASSWORD;
use crate::fake_device::{data::object_table::CPINTable, MSID_PASSWORD};
use crate::spec::opal::admin::*;
use crate::spec::{self, objects::CPIN};

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
