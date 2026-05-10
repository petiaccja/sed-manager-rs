use sed_packet::ObjectUid;
use sed_spec_macros::{DetokenizeStruct, FieldList, Object, TokenizeField, TokenizeStruct};

use crate::objects::KAes256Ref;
use crate::preconfig::core::shared::table_id;
use crate::types::SymmetricModeMedia;

#[derive(Debug, Clone, Default, PartialEq, Eq, Object, TokenizeStruct, DetokenizeStruct, FieldList, TokenizeField)]
#[object(table=table_id::K_AES_256)]
pub struct KAes256 {
    pub uid: Option<KAes256Ref>,
    pub name: Option<String>,
    pub common_name: Option<String>,
    pub key: Option<[u8; 64]>,
    pub mode: Option<SymmetricModeMedia>,
}

impl ObjectUid for KAes256 {
    fn uid(&self) -> Option<Self::Ref> {
        self.uid
    }
}
