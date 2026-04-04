use std::collections::HashSet;

use crate::objects::MbrControlRef;
use crate::preconfig::core::shared::table_id;
use crate::types::ResetType;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[sed_spec_macros::object(table=table_id::MBR_CONTROL)]
pub struct MbrControl {
    pub uid: MbrControlRef,
    pub enable: bool,
    pub done: bool,
    pub done_on_reset: HashSet<ResetType>,
}
