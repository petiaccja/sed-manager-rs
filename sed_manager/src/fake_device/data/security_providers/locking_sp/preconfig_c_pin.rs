use crate::spec::opal::locking::*;
use crate::{fake_device::data::object_table::CPINTable, spec::objects::CPIN};

use super::{ADMIN_IDX, USER_IDX};

pub fn preconfig_c_pin() -> CPINTable {
    let mut items = vec![];

    for index in ADMIN_IDX {
        items.push(CPIN {
            uid: c_pin::ADMIN.nth(index).unwrap(),
            pin: "8965823nz987gt346".into(),
            ..Default::default()
        });
    }

    for index in USER_IDX {
        items.push(CPIN {
            uid: c_pin::USER.nth(index).unwrap(),
            pin: "8965823nz987gt346".into(),
            ..Default::default()
        });
    }

    items.into_iter().collect()
}
