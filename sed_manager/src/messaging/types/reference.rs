use crate::messaging::{uid::UID, value::Value};

#[derive(PartialEq, Eq, Clone, Copy, Debug, Hash, PartialOrd, Ord)]
pub struct RowReference(u64);

#[derive(PartialEq, Eq, Clone, Copy, Debug, Hash, PartialOrd, Ord)]
pub struct RestrictedRowReference<const TABLE: u64>(u64);

#[derive(PartialEq, Eq, Clone, Copy, Debug, Hash, PartialOrd, Ord)]
pub struct RestrictedObjectReference<const TABLE: u64>(UID);

#[derive(PartialEq, Eq, Clone, Copy, Debug, Hash, PartialOrd, Ord)]
pub struct ObjectReference(UID);

#[derive(PartialEq, Eq, Clone, Copy, Debug, Hash, PartialOrd, Ord)]
pub struct TableReference(UID);

#[derive(PartialEq, Eq, Clone, Copy, Debug, Hash, PartialOrd, Ord)]
pub struct ByteTableReference(UID);

#[derive(PartialEq, Eq, Clone, Copy, Debug, Hash, PartialOrd, Ord)]
pub struct ObjectTableReference(UID);

macro_rules! impl_uid_reference {
    ($name:ty $(, $generic_name:ident: $generic_ty:ty)?) => {
        impl $(<const $generic_name: $generic_ty>)? $name {
            pub fn new(uid: UID) -> Self {
                Self(uid)
            }
        }

        impl $(<const $generic_name: $generic_ty>)? From<UID> for $name {
            fn from(value: UID) -> Self {
                Self::new(value)
            }
        }

        impl $(<const $generic_name: $generic_ty>)? From<$name > for UID {
            fn from(value: $name ) -> Self {
                value.0
            }
        }

        impl $(<const $generic_name: $generic_ty>)? TryFrom<Value> for $name {
            type Error = Value;
            fn try_from(value: Value) -> Result<Self, Self::Error> {
                Ok(Self::new(UID::try_from(value)?))
            }
        }

        impl $(<const $generic_name: $generic_ty>)? From<$name > for Value {
            fn from(value: $name) -> Self {
                Value::from(value.0)
            }
        }
    };

}

impl_uid_reference!(RestrictedObjectReference<TABLE>, TABLE: u64);
impl_uid_reference!(ObjectReference);
impl_uid_reference!(TableReference);
impl_uid_reference!(ByteTableReference);
impl_uid_reference!(ObjectTableReference);
