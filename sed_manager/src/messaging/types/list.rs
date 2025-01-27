use std::ops::{Deref, DerefMut};

use crate::messaging::value::Value;

pub struct List<Item>(pub Vec<Item>);

impl<Item> List<Item> {
    pub fn new() -> Self {
        Self(Vec::new())
    }
    pub fn into_vec(self) -> Vec<Item> {
        self.0
    }
}

impl<Item> Deref for List<Item> {
    type Target = Vec<Item>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<Item> DerefMut for List<Item> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<Item> From<Vec<Item>> for List<Item> {
    fn from(value: Vec<Item>) -> Self {
        Self(value)
    }
}

impl<Item> From<List<Item>> for Value
where
    Value: From<Item>,
{
    fn from(value: List<Item>) -> Self {
        let converted: Vec<_> = value.into_vec().into_iter().map(|item| Value::from(item)).collect();
        <Value as From<crate::messaging::value::List>>::from(converted)
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
