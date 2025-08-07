//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::ops::{Add, Bound, RangeBounds, Sub};
use sed_manager_macros::StructType;

use crate::messaging::uid::{TableUID, UID};
use crate::messaging::value::{Bytes, List, Value};
use crate::spec::basic_types::TableReference;

/// Specifies a part of an object table or byte table.
///
/// The only valid configurations for a [`CellBlock`] are the following:
///
/// | MethodID | InvokingID | Table      | StartRow   | EndRow    | StartColumn | EndColumn |
/// |----------|------------|------------|------------|-----------|-------------|-----------|
/// | *        | b_table    | ---        | * (0)      | * (∞)     | ---         | ---       |
/// | ~Get     | *          | b_table    | * (0)      | * (∞)     | ---         | ---       |
/// | *        | o_table    | ---        | object     | ---       | * (0)       | * (∞)     |
/// | ~Get     | *          | o_table    | object     | ---       | * (0)       | * (∞)     |
/// | *        | object     | ---        | ---        | ---       | * (0)       | * (∞)     |
#[derive(StructType, PartialEq, Eq, Clone, Debug, Default)]
pub struct CellBlock {
    pub table: Option<TableReference>,
    pub start_row: Option<u64>, // This is a typeOr{ uinteger | UID }, but it's encoded plain, without name-value pair.
    pub end_row: Option<u64>,
    pub start_column: Option<u16>,
    pub end_column: Option<u16>,
}

pub struct ObjectCellBlock {
    pub table: TableUID,
    pub object: UID,
    pub start_column: Option<u16>,
    pub end_column: Option<u16>,
}

pub struct ByteCellBlock {
    pub table: TableUID,
    pub start_byte: Option<u64>,
    pub end_byte: Option<u64>,
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

    /// Get the table the method invocation should operate on.
    ///
    /// Does not fully validate the [`CellBlock`] and the method call, so may return a table
    /// even if the method call is invalid. If a table is returned, it always is the table
    /// that the method call should operate on, regardless of the validity of the call.
    pub fn target_table(&self, invoking_id: UID) -> Option<TableUID> {
        if let Ok(table) = TableUID::try_from(invoking_id) {
            Some(table)
        } else if let Some(Ok(table)) = invoking_id.containing_table().map(|table| TableUID::try_from(table)) {
            Some(table)
        } else if let Some(table) = self.table {
            Some(table)
        } else {
            None
        }
    }

    pub fn try_into_object(self, invoking_id: UID) -> Result<ObjectCellBlock, Self> {
        // Valid object configurations:
        //
        // | MethodID | InvokingID | Table      | StartRow   | EndRow    | StartColumn | EndColumn |
        // |----------|------------|------------|------------|-----------|-------------|-----------|
        // | *        | o_table    | ---        | object     | ---       | * (0)       | * (∞)     |
        // | ~Get     | *          | o_table    | object     | ---       | * (0)       | * (∞)     |
        // | *        | object     | ---        | ---        | ---       | * (0)       | * (∞)     |
        let inv_table = TableUID::try_from(invoking_id).ok();
        let explicit_table = self.table;
        let inv_object = invoking_id.is_object().then_some(invoking_id);
        let explicit_object = self.start_row.map(|value| UID::from(value)).filter(|uid| uid.is_object());
        let (table, object) = match (inv_table, explicit_table, inv_object, explicit_object) {
            (Some(table), None, None, Some(object)) => (table, object),
            (_, Some(table), None, Some(object)) => (table, object),
            (None, None, Some(object), None) => {
                (object.containing_table().map(|uid| TableUID::new(uid.as_u64())).unwrap_or(TableUID::null()), object)
            }
            _ => return Err(self),
        };
        if self.end_row.is_some() {
            Err(self)
        } else if Some(table.as_uid()) == object.containing_table() {
            Ok(ObjectCellBlock { table, object, start_column: self.start_column, end_column: self.end_column })
        } else {
            Err(self)
        }
    }

    pub fn try_into_byte(self, invoking_id: UID) -> Result<ByteCellBlock, Self> {
        // Valid byte configurations:
        //
        // | MethodID | InvokingID | Table      | StartRow   | EndRow    | StartColumn | EndColumn |
        // |----------|------------|------------|------------|-----------|-------------|-----------|
        // | *        | b_table    | ---        | * (0)      | * (∞)     | ---         | ---       |
        // | ~Get     | *          | b_table    | * (0)      | * (∞)     | ---         | ---       |
        let inv_table = TableUID::try_from(invoking_id).ok();
        let explicit_table = self.table;
        let table = match (inv_table, explicit_table) {
            (Some(table), None) => table,
            (_, Some(table)) => table,
            _ => return Err(self),
        };
        if self.start_column.is_some() || self.end_column.is_some() {
            Err(self)
        } else {
            Ok(ByteCellBlock { table, start_byte: self.start_row, end_byte: self.end_row })
        }
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

#[cfg(test)]
mod tests {
    use crate::spec::core::authority;
    use crate::spec::{invoking_id, table_id};

    use super::*;

    #[test]
    fn cell_block_target_table() {
        let t1 = table_id::AUTHORITY;
        let t2 = table_id::TEMPLATE;
        let t1_o1 = authority::SID;
        let empty = CellBlock { table: None, start_row: None, end_row: None, start_column: None, end_column: None };
        let cases = [
            // Call on table
            (t1.as_uid(), CellBlock { table: None, start_row: None, ..empty }, Some(t1)),
            (t1.as_uid(), CellBlock { table: None, start_row: Some(t1_o1.as_u64()), ..empty }, Some(t1)),
            (t1.as_uid(), CellBlock { table: Some(t1), start_row: None, ..empty }, Some(t1)),
            (t2.as_uid(), CellBlock { table: Some(t1), start_row: None, ..empty }, Some(t2)),
            // Call on object
            (t1_o1.as_uid(), CellBlock { table: None, start_row: None, ..empty }, Some(t1)),
            (t1_o1.as_uid(), CellBlock { table: Some(t1), start_row: None, ..empty }, Some(t1)),
            (t1_o1.as_uid(), CellBlock { table: Some(t2), start_row: None, ..empty }, Some(t1)),
            // Call on ThisSP
            (invoking_id::THIS_SP, CellBlock { table: None, start_row: None, ..empty }, None),
            (invoking_id::THIS_SP, CellBlock { table: Some(t1), start_row: None, ..empty }, Some(t1)),
            (invoking_id::THIS_SP, CellBlock { table: None, start_row: Some(t1_o1.as_u64()), ..empty }, None),
        ];
        for (i, (invoking_id, cell_block, expected)) in cases.iter().enumerate() {
            assert_eq!(cell_block.target_table(*invoking_id), *expected, "case #{i}");
        }
    }

    #[test]
    fn cell_block_into_object() {
        let t1 = table_id::AUTHORITY;
        let t2 = table_id::TEMPLATE;
        let t1u = t1.as_uid();
        let t2u = t2.as_uid();
        let t1_o1 = authority::SID;
        let t1_o1u = t1_o1.as_uid();
        let this_sp = invoking_id::THIS_SP;
        let empty = CellBlock { table: None, start_row: None, end_row: None, start_column: None, end_column: None };
        let cases = [
            // Call on table
            (t1u, CellBlock { table: None, start_row: None, ..empty }, None),
            (t1u, CellBlock { table: None, start_row: Some(t1_o1.as_u64()), ..empty }, Some((t1, t1_o1u))),
            (t1u, CellBlock { table: Some(t1), start_row: None, ..empty }, None),
            (t1u, CellBlock { table: Some(t1), start_row: Some(t1_o1.as_u64()), ..empty }, Some((t1, t1_o1u))),
            (t2u, CellBlock { table: None, start_row: Some(t1_o1.as_u64()), ..empty }, None),
            (t2u, CellBlock { table: Some(t1), start_row: None, ..empty }, None),
            (t2u, CellBlock { table: Some(t1), start_row: Some(t1_o1.as_u64()), ..empty }, Some((t1, t1_o1u))),
            // Call on object
            (t1_o1u, CellBlock { table: None, start_row: None, ..empty }, Some((t1, t1_o1u))),
            (t1_o1u, CellBlock { table: None, start_row: Some(t1_o1.as_u64()), ..empty }, None),
            (t1_o1u, CellBlock { table: Some(t1), start_row: None, ..empty }, None),
            (t1_o1u, CellBlock { table: Some(t1), start_row: Some(t1_o1.as_u64()), ..empty }, None),
            // Call on ThisSP
            (this_sp, CellBlock { table: None, start_row: None, ..empty }, None),
            (this_sp, CellBlock { table: None, start_row: Some(t1_o1.as_u64()), ..empty }, None),
            (this_sp, CellBlock { table: Some(t1), start_row: None, ..empty }, None),
            (this_sp, CellBlock { table: Some(t1), start_row: Some(t1_o1.as_u64()), ..empty }, Some((t1, t1_o1u))),
        ];
        for (i, (invoking_id, cell_block, expected)) in cases.iter().enumerate() {
            let result = cell_block.clone().try_into_object(*invoking_id);
            let result_cmp = result.map(|cb| (cb.table, cb.object));
            assert_eq!(result_cmp.ok(), *expected, "case #{i}");
        }
    }

    #[test]
    fn cell_block_into_byte() {
        let t1 = table_id::MBR;
        let t2 = table_id::AUTHORITY;
        let r1 = 2635427;
        let t1u = t1.as_uid();
        let t2u = t2.as_uid();
        let t2_o1 = authority::SID;
        let t2_o1u = t2_o1.as_uid();
        let this_sp = invoking_id::THIS_SP;
        let empty = CellBlock { table: None, start_row: None, end_row: None, start_column: None, end_column: None };
        let cases = [
            // Call on table
            (t1u, CellBlock { table: None, start_row: None, ..empty }, Some(t1)),
            (t1u, CellBlock { table: None, start_row: Some(r1), ..empty }, Some(t1)),
            (t1u, CellBlock { table: Some(t1), start_row: None, ..empty }, Some(t1)),
            (t1u, CellBlock { table: Some(t1), start_row: Some(r1), ..empty }, Some(t1)),
            (t2u, CellBlock { table: Some(t1), start_row: None, ..empty }, Some(t1)),
            (t2u, CellBlock { table: Some(t1), start_row: Some(r1), ..empty }, Some(t1)),
            // Call on object
            (t2_o1u, CellBlock { table: None, start_row: None, ..empty }, None),
            (t2_o1u, CellBlock { table: None, start_row: Some(r1), ..empty }, None),
            (t2_o1u, CellBlock { table: Some(t1), start_row: None, ..empty }, Some(t1)),
            (t2_o1u, CellBlock { table: Some(t1), start_row: Some(r1), ..empty }, Some(t1)),
            // Call on ThisSP
            (this_sp, CellBlock { table: None, start_row: None, ..empty }, None),
            (this_sp, CellBlock { table: None, start_row: Some(r1), ..empty }, None),
            (this_sp, CellBlock { table: Some(t1), start_row: None, ..empty }, Some(t1)),
            (this_sp, CellBlock { table: Some(t1), start_row: Some(r1), ..empty }, Some(t1)),
        ];
        for (i, (invoking_id, cell_block, expected)) in cases.iter().enumerate() {
            let result = cell_block.clone().try_into_byte(*invoking_id);
            let result_cmp = result.map(|cb| cb.table);
            assert_eq!(result_cmp.ok(), *expected, "case #{i}");
        }
    }
}
