use core::ops::{Deref, DerefMut};
use std::collections::BTreeSet;

use crate::messaging::value::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Set<Item: Ord>(pub BTreeSet<Item>);

impl<Item: Ord> Set<Item> {
    pub fn new() -> Self {
        Self(BTreeSet::new())
    }
    pub fn into_set(self) -> BTreeSet<Item> {
        self.0
    }
}

impl<Item: Ord> Deref for Set<Item> {
    type Target = BTreeSet<Item>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<Item: Ord> DerefMut for Set<Item> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<Item: Ord> From<BTreeSet<Item>> for Set<Item> {
    fn from(value: BTreeSet<Item>) -> Self {
        Self(value)
    }
}

impl<Item: Ord> From<Set<Item>> for Value
where
    Value: From<Item>,
{
    fn from(value: Set<Item>) -> Self {
        let converted: Vec<_> = value.into_iter().map(|item| Value::from(item)).collect();
        <Value as From<crate::messaging::value::List>>::from(converted)
    }
}

impl<Item: TryFrom<Value> + Ord> TryFrom<Value> for Set<Item> {
    type Error = Value;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let items = crate::messaging::value::List::try_from(value.clone())?;
        let converted: Result<BTreeSet<_>, _> = items.into_iter().map(|item| Item::try_from(item)).collect();
        match converted {
            Ok(converted) => Ok(converted.into()),
            Err(_) => Err(value),
        }
    }
}

impl<Item: Ord> IntoIterator for Set<Item> {
    type Item = Item;
    type IntoIter = std::collections::btree_set::IntoIter<Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<Item: Ord> FromIterator<Item> for Set<Item> {
    fn from_iter<T: IntoIterator<Item = Item>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<Item: Ord, const N: usize> From<[Item; N]> for Set<Item> {
    fn from(value: [Item; N]) -> Self {
        value.into_iter().collect()
    }
}
