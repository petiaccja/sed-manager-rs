use crate::objects::KAes256Ref;
use crate::preconfig::core::shared::table_id;
use crate::types::SymmetricModeMedia;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[sed_spec_macros::object(table=table_id::K_AES_256)]
pub struct KAes256 {
    pub uid: KAes256Ref,
    pub name: String,
    pub common_name: String,
    pub key: [u8; 64],
    pub mode: SymmetricModeMedia,
}
