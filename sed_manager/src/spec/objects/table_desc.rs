use as_array::AsArray;

use crate::{
    messaging::uid::UID,
    spec::column_types::{ColumnRef, Name, TableDescRef, TableKind, TemplateRef},
};

use super::cell::Cell;

#[derive(AsArray)]
#[as_array_traits(Cell)]
pub struct TableDesc {
    pub uid: TableDescRef,
    pub name: Name,
    pub common_name: Name,
    pub template_id: TemplateRef,
    pub kind: TableKind,
    pub column: ColumnRef,
    pub num_columns: u32,
    pub rows: u32,
    pub rows_free: u32,
    pub row_bytes: u32,
    pub last_id: UID,
    pub min_size: u32,
    pub max_size: u32,
}

impl TableDesc {
    pub const UID: u16 = 0x00;
    pub const NAME: u16 = 0x01;
    pub const COMMON_NAME: u16 = 0x02;
    pub const TEMPLATE_ID: u16 = 0x03;
    pub const KIND: u16 = 0x04;
    pub const COLUMN: u16 = 0x05;
    pub const NUM_COLUMNS: u16 = 0x06;
    pub const ROWS: u16 = 0x07;
    pub const ROWS_FREE: u16 = 0x08;
    pub const ROW_BYTES: u16 = 0x09;
    pub const LAST_ID: u16 = 0x0A;
    pub const MIN_SIZE: u16 = 0x0B;
    pub const MAX_SIZE: u16 = 0x0C;
}

impl Default for TableDesc {
    fn default() -> Self {
        Self {
            uid: TableDescRef::null(),
            name: Name::default(),
            common_name: Name::default(),
            template_id: TemplateRef::null(),
            kind: TableKind::Unknown,
            column: ColumnRef::null(),
            num_columns: 0,
            rows: 0,
            rows_free: 0,
            row_bytes: 0,
            last_id: UID::null(),
            min_size: 0,
            max_size: 0,
        }
    }
}
