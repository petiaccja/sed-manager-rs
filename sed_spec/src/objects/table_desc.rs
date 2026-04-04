use sed_packet::Uid;

use crate::objects::{ColumnRef, TableDescRef, TemplateRef};
use crate::preconfig::core::shared::table_id;
use crate::types::TableKind;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[sed_spec_macros::object(table=table_id::TABLE)]
pub struct TableDesc {
    pub uid: TableDescRef,
    pub name: String,
    pub common_name: String,
    pub template_id: TemplateRef,
    pub kind: TableKind,
    pub column: ColumnRef,
    pub num_columns: u32,
    pub rows: u32,
    pub rows_free: u32,
    pub row_bytes: u32,
    pub last_id: Uid,
    pub min_size: u32,
    pub max_size: u32,
}
