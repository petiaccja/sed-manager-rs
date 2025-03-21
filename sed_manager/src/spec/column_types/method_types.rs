//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::ops::{Add, Bound, RangeBounds, Sub};
use sed_manager_macros::StructType;

use crate::messaging::uid::{TableUID, UID};
use crate::messaging::value::{Bytes, List, Value};
use crate::spec::basic_types::TableReference;

#[derive(StructType, PartialEq, Eq, Clone, Debug, Default)]
pub struct CellBlock {
    pub table: Option<TableReference>,
    pub start_row: Option<u64>, // This is a typeOr{ uinteger | UID }, but it's encoded plain, without name-value pair.
    pub end_row: Option<u64>,
    pub start_column: Option<u16>,
    pub end_column: Option<u16>,
}

/// Result returned by the Authenticate method.
/// I'm guessing it's not encoded as an NVP like regular typeOr{} objects, but simply as plain data.
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum BoolOrBytes {
    Bool(bool),
    Bytes(Bytes),
}

/// Represents the result of the Get method.
/// According to the TCG examples, it's not encoded as an NVP like regular typeOr{} objects, but simply as plain data.
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum BytesOrRowValues {
    Bytes(Bytes),
    RowValues(List),
}

impl CellBlock {
    pub fn object(columns: impl RangeBounds<u16>) -> Self {
        let (start_column, end_column) = Self::map_bounds(columns);
        Self { table: None, start_row: None, end_row: None, start_column, end_column }
    }

    pub fn object_with_table(object: UID, columns: impl RangeBounds<u16>) -> Self {
        let table = object.containing_table().map(|uid| uid.try_into().unwrap());
        let (start_column, end_column) = Self::map_bounds(columns);
        Self { table, start_row: Some(object.as_u64()), end_row: None, start_column, end_column }
    }

    pub fn bytes(bytes: impl RangeBounds<u64>) -> Self {
        let (start_row, end_row) = Self::map_bounds(bytes);
        Self { table: None, start_row, end_row, start_column: None, end_column: None }
    }

    pub fn bytes_with_table(table: TableUID, bytes: impl RangeBounds<u64>) -> Self {
        let (start_row, end_row) = Self::map_bounds(bytes);
        Self { table: Some(table), start_row, end_row, start_column: None, end_column: None }
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
            Bound::Excluded(x) => Some(core::cmp::max(1u8.into(), *x) - 1u8.into()),
            Bound::Included(x) => Some(*x),
        };
        (start, end)
    }
}

impl TryFrom<Value> for BoolOrBytes {
    type Error = Value;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let maybe_bool = bool::try_from(value).map(|x| BoolOrBytes::Bool(x));
        let value = match maybe_bool {
            Ok(x) => return Ok(x),
            Err(v) => v,
        };
        Bytes::try_from(value).map(|x| BoolOrBytes::Bytes(x))
    }
}

impl From<BoolOrBytes> for Value {
    fn from(value: BoolOrBytes) -> Self {
        match value {
            BoolOrBytes::Bool(x) => x.into(),
            BoolOrBytes::Bytes(x) => x.into(),
        }
    }
}

impl TryFrom<Value> for BytesOrRowValues {
    type Error = Value;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let maybe_bool = Bytes::try_from(value).map(|x| BytesOrRowValues::Bytes(x));
        let value = match maybe_bool {
            Ok(x) => return Ok(x),
            Err(v) => v,
        };
        List::try_from(value).map(|x| BytesOrRowValues::RowValues(x))
    }
}

impl From<BytesOrRowValues> for Value {
    fn from(value: BytesOrRowValues) -> Self {
        match value {
            BytesOrRowValues::Bytes(x) => x.into(),
            BytesOrRowValues::RowValues(x) => x.into(),
        }
    }
}
