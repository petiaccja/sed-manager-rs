//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::{fake_device::data::object_table::MBRControlTable, spec::objects::MBRControl};

pub fn preconfig_mbr_control() -> MBRControlTable {
    [MBRControl { ..Default::default() }].into_iter().collect()
}
