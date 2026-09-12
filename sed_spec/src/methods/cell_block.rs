//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::ops::{Add, Bound, RangeBounds, Sub};

use sed_packet::token::{Detokenize, Detokenizer, MessageError as _, TokenType, Tokenize, Tokenizer};
use sed_packet::{ObjectRef, TableRef, Uid};
use sed_spec_macros::{DetokenizeStruct, TokenizeStruct};

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
#[derive(PartialEq, Eq, Clone, Debug, DetokenizeStruct, TokenizeStruct)]
pub struct CellBlock {
    pub table: Option<TableRef>,
    pub start_row: Option<UidOrU64>,
    pub end_row: Option<u64>,
    pub start_column: Option<u16>,
    pub end_column: Option<u16>,
}

/// This is a typeOr{ Uid | uinteger }, but it's encoded plain, without name-value pair.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum UidOrU64 {
    Uid(Uid),
    U64(u64),
}

impl TryFrom<UidOrU64> for u64 {
    type Error = UidOrU64;

    fn try_from(value: UidOrU64) -> Result<Self, Self::Error> {
        match value {
            UidOrU64::Uid(_) => Err(value),
            UidOrU64::U64(integer) => Ok(integer),
        }
    }
}

impl TryFrom<UidOrU64> for Uid {
    type Error = UidOrU64;

    fn try_from(value: UidOrU64) -> Result<Self, Self::Error> {
        match value {
            UidOrU64::Uid(uid) => Ok(uid),
            UidOrU64::U64(_) => Err(value),
        }
    }
}

impl From<Uid> for UidOrU64 {
    fn from(value: Uid) -> Self {
        Self::Uid(value)
    }
}

impl From<TableRef> for UidOrU64 {
    fn from(value: TableRef) -> Self {
        Self::Uid(value.to_uid())
    }
}

impl<const TABLE: u64> From<ObjectRef<TABLE>> for UidOrU64 {
    fn from(value: ObjectRef<TABLE>) -> Self {
        Self::Uid(value.to_uid())
    }
}

impl From<u64> for UidOrU64 {
    fn from(value: u64) -> Self {
        Self::U64(value)
    }
}

impl Tokenize for UidOrU64 {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        match self {
            UidOrU64::Uid(uid) => uid.tokenize(tokenizer),
            UidOrU64::U64(number) => number.tokenize(tokenizer),
        }
    }
}

impl Detokenize for UidOrU64 {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        match detokenizer.peek_kind()? {
            TokenType::Integer { .. } => u64::detokenize(detokenizer).map(|value| Self::U64(value)),
            TokenType::Bytes => Uid::detokenize(detokenizer).map(|value| Self::Uid(value)),
            _ => Err(D::Error::message("expected either an Uid or an unsigned integer")),
        }
    }
}

pub struct ObjectCellBlock {
    pub table: TableRef,
    pub object: Uid,
    pub start_column: Option<u16>,
    pub end_column: Option<u16>,
}

pub struct ByteCellBlock {
    pub table: TableRef,
    pub start_byte: Option<u64>,
    pub end_byte: Option<u64>,
}

impl CellBlock {
    pub fn object(columns: impl RangeBounds<u16>) -> Self {
        let (start_column, end_column) = Self::map_bounds(columns);
        Self { table: None, start_row: None, end_row: None, start_column, end_column }
    }

    pub fn object_with_table(object: Uid, columns: impl RangeBounds<u16>) -> Self {
        let table = object.containing_table().map(|uid| uid.try_into().unwrap());
        let (start_column, end_column) = Self::map_bounds(columns);
        Self { table, start_row: Some(UidOrU64::Uid(object)), end_row: None, start_column, end_column }
    }

    pub fn bytes(bytes: impl RangeBounds<u64>) -> Self {
        let (start_row, end_row) = Self::map_bounds(bytes);
        let start_row = start_row.map(|start_row| UidOrU64::U64(start_row));
        Self { table: None, start_row, end_row, start_column: None, end_column: None }
    }

    pub fn bytes_with_table(table: TableRef, bytes: impl RangeBounds<u64>) -> Self {
        let (start_row, end_row) = Self::map_bounds(bytes);
        let start_row = start_row.map(|start_row| UidOrU64::U64(start_row));
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
    pub fn get_target_table(&self, invoking_id: Uid) -> Option<TableRef> {
        if let Ok(table) = TableRef::try_from(invoking_id) {
            Some(table)
        } else if let Some(Ok(table)) = invoking_id.containing_table().map(|table| TableRef::try_from(table)) {
            Some(table)
        } else if let Some(table) = self.table {
            Some(table)
        } else {
            None
        }
    }

    pub fn try_into_object(self, invoking_id: Uid) -> Result<ObjectCellBlock, Self> {
        // Valid object configurations:
        //
        // | MethodID | InvokingID | Table      | StartRow   | EndRow    | StartColumn | EndColumn |
        // |----------|------------|------------|------------|-----------|-------------|-----------|
        // | *        | o_table    | ---        | object     | ---       | * (0)       | * (∞)     |
        // | ~Get     | *          | o_table    | object     | ---       | * (0)       | * (∞)     |
        // | *        | object     | ---        | ---        | ---       | * (0)       | * (∞)     |
        let inv_table = TableRef::try_from(invoking_id).ok();
        let explicit_table = self.table;
        let inv_object = invoking_id.is_object().then_some(invoking_id);
        let Ok(explicit_object) = self.start_row.map(|value| Uid::try_from(value)).transpose() else {
            return Err(self);
        };
        let (table, object) = match (inv_table, explicit_table, inv_object, explicit_object) {
            (Some(table), None, None, Some(object)) => (table, object),
            (_, Some(table), None, Some(object)) => (table, object),
            (None, None, Some(object), None) => (
                TableRef::new(object.containing_table().expect("inv_object is check to be an object").to_u64()),
                object,
            ),
            _ => return Err(self),
        };
        if self.end_row.is_some() {
            Err(self)
        } else if Some(table.to_uid()) == object.containing_table() {
            Ok(ObjectCellBlock { table, object, start_column: self.start_column, end_column: self.end_column })
        } else {
            Err(self)
        }
    }

    pub fn try_into_byte(self, invoking_id: Uid) -> Result<ByteCellBlock, Self> {
        // Valid byte configurations:
        //
        // | MethodID | InvokingID | Table      | StartRow   | EndRow    | StartColumn | EndColumn |
        // |----------|------------|------------|------------|-----------|-------------|-----------|
        // | *        | b_table    | ---        | * (0)      | * (∞)     | ---         | ---       |
        // | ~Get     | *          | b_table    | * (0)      | * (∞)     | ---         | ---       |
        let inv_table = TableRef::try_from(invoking_id).ok();
        let explicit_table = self.table;
        let table = match (inv_table, explicit_table) {
            (Some(table), None) => table,
            (_, Some(table)) => table,
            _ => return Err(self),
        };
        if self.start_column.is_some() || self.end_column.is_some() {
            Err(self)
        } else {
            match self.start_row.map(|value| u64::try_from(value)).transpose() {
                Ok(start_byte) => Ok(ByteCellBlock { table, start_byte, end_byte: self.end_row }),
                Err(_) => Err(self),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::preconfig::core::shared::{authority, invoking_id, table_id};

    use super::*;

    #[test]
    fn cell_block_target_table() {
        let t1 = table_id::AUTHORITY;
        let t2 = table_id::TEMPLATE;
        let t1_o1 = authority::SID;
        let empty = CellBlock { table: None, start_row: None, end_row: None, start_column: None, end_column: None };
        let cases = [
            // Call on table
            (t1.to_uid(), CellBlock { table: None, start_row: None, ..empty }, Some(t1)),
            (t1.to_uid(), CellBlock { table: None, start_row: Some(t1_o1.into()), ..empty }, Some(t1)),
            (t1.to_uid(), CellBlock { table: Some(t1), start_row: None, ..empty }, Some(t1)),
            (t2.to_uid(), CellBlock { table: Some(t1), start_row: None, ..empty }, Some(t2)),
            // Call on object
            (t1_o1.to_uid(), CellBlock { table: None, start_row: None, ..empty }, Some(t1)),
            (t1_o1.to_uid(), CellBlock { table: Some(t1), start_row: None, ..empty }, Some(t1)),
            (t1_o1.to_uid(), CellBlock { table: Some(t2), start_row: None, ..empty }, Some(t1)),
            // Call on ThisSP
            (invoking_id::THIS_SP, CellBlock { table: None, start_row: None, ..empty }, None),
            (invoking_id::THIS_SP, CellBlock { table: Some(t1), start_row: None, ..empty }, Some(t1)),
            (invoking_id::THIS_SP, CellBlock { table: None, start_row: Some(t1_o1.into()), ..empty }, None),
        ];
        for (i, (invoking_id, cell_block, expected)) in cases.iter().enumerate() {
            assert_eq!(cell_block.get_target_table(*invoking_id), *expected, "case #{i}");
        }
    }

    #[test]
    fn cell_block_into_object() {
        let t1 = table_id::AUTHORITY;
        let t2 = table_id::TEMPLATE;
        let t1u = t1.to_uid();
        let t2u = t2.to_uid();
        let t1_o1 = authority::SID;
        let t1_o1u = t1_o1.to_uid();
        let this_sp = invoking_id::THIS_SP;
        let empty = CellBlock { table: None, start_row: None, end_row: None, start_column: None, end_column: None };
        let cases = [
            // Call on table
            (t1u, CellBlock { table: None, start_row: None, ..empty }, None),
            (t1u, CellBlock { table: None, start_row: Some(t1_o1.into()), ..empty }, Some((t1, t1_o1u))),
            (t1u, CellBlock { table: Some(t1), start_row: None, ..empty }, None),
            (t1u, CellBlock { table: Some(t1), start_row: Some(t1_o1.into()), ..empty }, Some((t1, t1_o1u))),
            (t2u, CellBlock { table: None, start_row: Some(t1_o1.into()), ..empty }, None),
            (t2u, CellBlock { table: Some(t1), start_row: None, ..empty }, None),
            (t2u, CellBlock { table: Some(t1), start_row: Some(t1_o1.into()), ..empty }, Some((t1, t1_o1u))),
            // Call on object
            (t1_o1u, CellBlock { table: None, start_row: None, ..empty }, Some((t1, t1_o1u))),
            (t1_o1u, CellBlock { table: None, start_row: Some(t1_o1.into()), ..empty }, None),
            (t1_o1u, CellBlock { table: Some(t1), start_row: None, ..empty }, None),
            (t1_o1u, CellBlock { table: Some(t1), start_row: Some(t1_o1.into()), ..empty }, None),
            // Call on ThisSP
            (this_sp, CellBlock { table: None, start_row: None, ..empty }, None),
            (this_sp, CellBlock { table: None, start_row: Some(t1_o1.into()), ..empty }, None),
            (this_sp, CellBlock { table: Some(t1), start_row: None, ..empty }, None),
            (this_sp, CellBlock { table: Some(t1), start_row: Some(t1_o1.into()), ..empty }, Some((t1, t1_o1u))),
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
        let t1u = t1.to_uid();
        let t2u = t2.to_uid();
        let t2_o1 = authority::SID;
        let t2_o1u = t2_o1.to_uid();
        let this_sp = invoking_id::THIS_SP;
        let empty = CellBlock { table: None, start_row: None, end_row: None, start_column: None, end_column: None };
        let cases = [
            // Call on table
            (t1u, CellBlock { table: None, start_row: None, ..empty }, Some(t1)),
            (t1u, CellBlock { table: None, start_row: Some(r1.into()), ..empty }, Some(t1)),
            (t1u, CellBlock { table: Some(t1), start_row: None, ..empty }, Some(t1)),
            (t1u, CellBlock { table: Some(t1), start_row: Some(r1.into()), ..empty }, Some(t1)),
            (t2u, CellBlock { table: Some(t1), start_row: None, ..empty }, Some(t1)),
            (t2u, CellBlock { table: Some(t1), start_row: Some(r1.into()), ..empty }, Some(t1)),
            // Call on object
            (t2_o1u, CellBlock { table: None, start_row: None, ..empty }, None),
            (t2_o1u, CellBlock { table: None, start_row: Some(r1.into()), ..empty }, None),
            (t2_o1u, CellBlock { table: Some(t1), start_row: None, ..empty }, Some(t1)),
            (t2_o1u, CellBlock { table: Some(t1), start_row: Some(r1.into()), ..empty }, Some(t1)),
            // Call on ThisSP
            (this_sp, CellBlock { table: None, start_row: None, ..empty }, None),
            (this_sp, CellBlock { table: None, start_row: Some(r1.into()), ..empty }, None),
            (this_sp, CellBlock { table: Some(t1), start_row: None, ..empty }, Some(t1)),
            (this_sp, CellBlock { table: Some(t1), start_row: Some(r1.into()), ..empty }, Some(t1)),
        ];
        for (i, (invoking_id, cell_block, expected)) in cases.iter().enumerate() {
            let result = cell_block.clone().try_into_byte(*invoking_id);
            let result_cmp = result.map(|cb| cb.table);
            assert_eq!(result_cmp.ok(), *expected, "case #{i}");
        }
    }
}
