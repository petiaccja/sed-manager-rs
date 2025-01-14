use std::collections::HashSet;
use std::ops::{Deref, DerefMut};

use crate::messaging::{
    uid::UID,
    value::{Bytes, Value},
};

pub struct MaxBytes<const LIMIT: usize> {
    bytes: Vec<u8>,
}

pub struct List<Item> {
    items: Vec<Item>,
}

pub struct RestrictedRowReference<const TABLE: u64> {
    row: u64,
}

pub struct RestrictedObjectReference<const TABLE: u64> {
    object: UID,
}

pub struct RowReference {
    row: u64,
}

pub struct ObjectReference {
    object: UID,
}

pub struct TableReference {
    table: UID,
}

pub struct ByteTableReference {
    table: UID,
}

pub struct ObjectTableReference {
    table: UID,
}

pub struct NamedValue<NameTy, ValueTy> {
    pub name: NameTy,
    pub value: ValueTy,
}

pub struct Set<T> {
    items: HashSet<T>,
}

impl<const LIMIT: usize> Deref for MaxBytes<LIMIT> {
    type Target = Vec<u8>;
    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

impl<const LIMIT: usize> From<MaxBytes<LIMIT>> for Vec<u8> {
    fn from(value: MaxBytes<LIMIT>) -> Self {
        value.bytes
    }
}

impl<const LIMIT: usize> From<Vec<u8>> for MaxBytes<LIMIT> {
    fn from(value: Vec<u8>) -> Self {
        Self { bytes: value }
    }
}

impl<const LIMIT: usize> From<MaxBytes<LIMIT>> for Value {
    fn from(value: MaxBytes<LIMIT>) -> Self {
        Value::from(value.bytes)
    }
}

impl<const LIMIT: usize> TryFrom<Value> for MaxBytes<LIMIT> {
    type Error = Value;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        Ok(Self { bytes: Bytes::try_from(value)? })
    }
}

impl<const LIMIT: usize> TryFrom<MaxBytes<LIMIT>> for String {
    type Error = MaxBytes<LIMIT>;
    fn try_from(value: MaxBytes<LIMIT>) -> Result<Self, Self::Error> {
        match String::from_utf8(value.bytes) {
            Ok(s) => Ok(s),
            Err(err) => Err(err.into_bytes().into()),
        }
    }
}

impl<const LIMIT: usize> From<String> for MaxBytes<LIMIT> {
    fn from(value: String) -> Self {
        Self { bytes: value.into_bytes() }
    }
}

impl<const LIMIT: usize> From<&str> for MaxBytes<LIMIT> {
    fn from(value: &str) -> Self {
        Self { bytes: value.as_bytes().into() }
    }
}

impl<Item> List<Item> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }
    pub fn into_vec(self) -> Vec<Item> {
        self.items
    }
}

impl<Item> Deref for List<Item> {
    type Target = Vec<Item>;
    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl<Item> DerefMut for List<Item> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.items
    }
}

impl<Item> From<Vec<Item>> for List<Item> {
    fn from(value: Vec<Item>) -> Self {
        Self { items: value }
    }
}

impl<Item: Into<Value>> From<List<Item>> for Value {
    fn from(value: List<Item>) -> Self {
        let converted: Vec<Value> = value.into_vec().into_iter().map(|item| item.into()).collect();
        Value::from(converted)
    }
}

impl<Item: TryFrom<Value>> TryFrom<Value> for List<Item> {
    type Error = Value;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let items = crate::messaging::value::List::try_from(value.clone())?;
        let converted: Result<Vec<_>, _> = items.into_iter().map(|item| Item::try_from(item)).collect();
        match converted {
            Ok(converted) => Ok(converted.into()),
            Err(_) => Err(value),
        }
    }
}

impl<NameTy: Into<Value>, ValueTy: Into<Value>> From<NamedValue<NameTy, ValueTy>> for Value {
    fn from(value: NamedValue<NameTy, ValueTy>) -> Self {
        Value::from(crate::messaging::value::Named { name: value.name.into(), value: value.value.into() })
    }
}

impl<NameTy: TryFrom<Value>, ValueTy: TryFrom<Value>> TryFrom<Value> for NamedValue<NameTy, ValueTy> {
    type Error = Value;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let named = crate::messaging::value::Named::try_from(value)?;
        if let (Ok(name), Ok(value)) = (NameTy::try_from(named.name.clone()), ValueTy::try_from(named.value.clone())) {
            Ok(NamedValue { name, value })
        } else {
            Err(Value::from(named))
        }
    }
}
