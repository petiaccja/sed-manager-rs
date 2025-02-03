use sed_manager_macros::StructType;
use std::ops::{Add, Bound, RangeBounds, Sub};

use super::{ByteTableReference, TableReference};

#[derive(StructType, PartialEq, Eq, Clone, Debug, Default)]
pub struct CellBlock {
    pub table: Option<TableReference>,
    pub start_row: Option<u64>, // This is a typeOr{ uinteger | UID }, but it's encoded plain, without name-value pair.
    pub end_row: Option<u64>,
    pub start_column: Option<u16>,
    pub end_column: Option<u16>,
}

impl CellBlock {
    pub fn object(columns: impl RangeBounds<u16>) -> Self {
        let (start_column, end_column) = Self::map_bounds(columns);
        Self { table: None, start_row: None, end_row: None, start_column, end_column }
    }

    pub fn byte_range(table: ByteTableReference, bytes: impl RangeBounds<u64>) -> Self {
        let (start_row, end_row) = Self::map_bounds(bytes);
        Self { table: Some(table.0.into()), start_row, end_row, start_column: None, end_column: None }
    }

    pub fn map_bounds<T>(bounds: impl RangeBounds<T>) -> (Option<T>, Option<T>)
    where
        T: Sized + Copy + Add<T, Output = T> + Sub<T, Output = T> + Ord + From<u8>,
    {
        let start = match bounds.start_bound() {
            Bound::Unbounded => None,
            Bound::Excluded(x) => Some(*x + 1u8.into()),
            Bound::Included(x) => Some(*x),
        };
        let end = match bounds.end_bound() {
            Bound::Unbounded => None,
            Bound::Excluded(x) => Some(std::cmp::max(1u8.into(), *x) - 1u8.into()),
            Bound::Included(x) => Some(*x),
        };
        (start, end)
    }
}
