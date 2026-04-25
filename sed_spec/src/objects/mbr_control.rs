use std::collections::HashSet;

use sed_spec_macros::{DetokenizeStruct, Object, TokenizeStruct, FieldList};

use crate::objects::MbrControlRef;
use crate::preconfig::core::shared::table_id;
use crate::types::ResetType;

#[derive(Debug, Clone, Default, PartialEq, Eq, Object, TokenizeStruct, DetokenizeStruct, FieldList)]
#[object(table=table_id::MBR_CONTROL)]
pub struct MbrControl {
    pub uid: Option<MbrControlRef>,
    pub enable: Option<bool>,
    pub done: Option<bool>,
    pub done_on_reset: Option<HashSet<ResetType>>,
}
