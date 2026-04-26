use sed_packet::{ObjectUid, Uid};

use sed_spec_macros::{DetokenizeStruct, FieldList, Object, TokenizeStruct};

use crate::objects::{ColumnRef, TableDescRef, TemplateRef};
use crate::preconfig::core::shared::table_id;
use crate::types::TableKind;

#[derive(Debug, Clone, Default, PartialEq, Eq, Object, TokenizeStruct, DetokenizeStruct, FieldList)]
#[object(table=table_id::TABLE)]
pub struct TableDesc {
    pub uid: Option<TableDescRef>,
    pub name: Option<String>,
    pub common_name: Option<String>,
    pub template_id: Option<TemplateRef>,
    pub kind: Option<TableKind>,
    pub column: Option<ColumnRef>,
    pub num_columns: Option<u32>,
    pub rows: Option<u32>,
    pub rows_free: Option<u32>,
    pub row_bytes: Option<u32>,
    pub last_id: Option<Uid>,
    pub min_size: Option<u32>,
    pub max_size: Option<u32>,
}

impl ObjectUid for TableDesc {
    fn uid(&self) -> Option<Self::Ref> {
        self.uid
    }
}
