//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::collections::HashSet;

use sed_packet::ObjectUid;
use sed_spec_macros::{DetokenizeStruct, FieldList, Object, TokenizeField, TokenizeStruct};

use crate::objects::MbrControlRef;
use crate::preconfig::core::shared::table_id;
use crate::types::ResetType;

#[derive(Debug, Clone, Default, PartialEq, Eq, Object, TokenizeStruct, DetokenizeStruct, FieldList, TokenizeField)]
#[object(table=table_id::MBR_CONTROL)]
pub struct MbrControl {
    pub uid: Option<MbrControlRef>,
    pub enable: Option<bool>,
    pub done: Option<bool>,
    pub done_on_reset: Option<HashSet<ResetType>>,
}

impl ObjectUid for MbrControl {
    fn uid(&self) -> Option<Self::Ref> {
        self.uid
    }
}
