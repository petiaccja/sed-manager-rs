use crate::{fake_device::data::object_table::MBRControlTable, spec::objects::MBRControl};

pub fn preconfig_mbr_control() -> MBRControlTable {
    [MBRControl { ..Default::default() }].into_iter().collect()
}
