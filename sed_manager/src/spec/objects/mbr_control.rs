use as_array::AsArray;

use crate::spec::column_types::{MBRControlRef, ResetType, ResetTypes};

use super::cell::Cell;

#[derive(AsArray)]
#[as_array_traits(Cell)]
pub struct MBRControl {
    pub uid: MBRControlRef,
    pub enable: bool,
    pub done: bool,
    pub done_on_reset: ResetTypes,
}

impl MBRControl {
    pub const UID: u16 = 0;
    pub const ENABLE: u16 = 1;
    pub const DONE: u16 = 2;
    pub const DONE_ON_RESET: u16 = 3;
}

impl Default for MBRControl {
    fn default() -> Self {
        Self {
            uid: crate::spec::core::mbr_control::MBR_CONTROL,
            enable: false,
            done: false,
            done_on_reset: [ResetType::PowerCycle].into_iter().collect(),
        }
    }
}
