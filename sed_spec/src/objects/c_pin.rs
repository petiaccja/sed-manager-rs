use sed_packet::{ObjectUid, TableRef};
use sed_spec_macros::{DetokenizeStruct, FieldList, Object, TokenizeStruct};
use smallvec::SmallVec;

use crate::objects::CPinRef;
use crate::preconfig::core::shared::table_id;

#[derive(Debug, Clone, Default, PartialEq, Eq, Object, TokenizeStruct, DetokenizeStruct, FieldList)]
#[object(table=table_id::C_PIN)]
pub struct CPin {
    pub uid: Option<CPinRef>,
    pub name: Option<String>,
    pub common_name: Option<String>,
    pub pin: Option<SmallVec<[u8; 32]>>,
    pub char_set: Option<TableRef>,
    pub try_limit: Option<u32>,
    pub tries: Option<u32>,
    pub persistence: Option<bool>,
}

impl ObjectUid for CPin {
    fn uid(&self) -> Option<Self::Ref> {
        self.uid
    }
}
