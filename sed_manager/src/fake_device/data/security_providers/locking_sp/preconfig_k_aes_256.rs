use crate::spec::opal::locking::*;
use crate::{fake_device::data::object_table::KAES256Table, spec::objects::KAES256};

use super::RANGE_IDX;

pub fn preconfig_k_aes_256() -> KAES256Table {
    let mut items = vec![KAES256 { uid: k_aes_256::GLOBAL_RANGE_KEY, ..Default::default() }];

    for index in RANGE_IDX {
        items.push(KAES256 { uid: k_aes_256::RANGE_KEY.nth(index).unwrap(), ..Default::default() });
    }

    items.into_iter().collect()
}
