//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::messaging::uid::{ObjectUID, TableUID, UID};
use sed_manager_macros::AliasType;

#[derive(PartialEq, Eq, Clone, Copy, Debug, Hash, PartialOrd, Ord, Default)]
pub struct RowReference(pub u64);

#[derive(PartialEq, Eq, Clone, Copy, Debug, Hash, PartialOrd, Ord, Default)]
pub struct RestrictedRowReference<const TABLE_MASK: u64>(pub u64);

pub type RestrictedObjectReference<const TABLE_MASK: u64> = ObjectUID<TABLE_MASK>;

#[derive(AliasType, PartialEq, Eq, Clone, Copy, Debug, Hash, PartialOrd, Ord, Default)]
pub struct ObjectReference(pub UID);

pub type TableReference = TableUID;

#[derive(AliasType, PartialEq, Eq, Clone, Copy, Debug, Hash, PartialOrd, Ord)]
pub struct ByteTableReference(pub TableReference);

#[derive(PartialEq, Eq, Clone, Copy, Debug, Hash, PartialOrd, Ord)]
pub struct ObjectTableReference(pub TableReference);
