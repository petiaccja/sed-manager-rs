//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::spec::objects::LockingRange;
use crate::spec::opal::locking::*;
use crate::{messaging::uid::ObjectUID, virtual_device::data::object_table::LockingTable};

use super::RANGE_IDX;

pub fn preconfig_locking() -> LockingTable {
    let mut items = vec![LockingRange {
        uid: locking::GLOBAL_RANGE,
        active_key: ObjectUID::new_other(k_aes_256::GLOBAL_RANGE_KEY),
        ..Default::default()
    }];

    for index in RANGE_IDX {
        items.push(LockingRange {
            uid: locking::RANGE.nth(index).unwrap(),
            active_key: ObjectUID::new_other(k_aes_256::RANGE_KEY.nth(index).unwrap()),
            ..Default::default()
        });
    }

    items.into_iter().collect()
}
