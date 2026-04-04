use sed_packet::TableRef;
use smallvec::SmallVec;

use crate::objects::CPinRef;
use crate::preconfig::core::shared::table_id;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[sed_spec_macros::object(table=table_id::C_PIN)]
pub struct CPin {
    pub uid: CPinRef,
    pub name: String,
    pub common_name: String,
    pub pin: SmallVec<[u8; 32]>,
    pub char_set: TableRef,
    pub try_limit: u32,
    pub tries: u32,
    pub persistence: bool,
}
