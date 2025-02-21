use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
};

use crate::messaging::uid::{ObjectUID, TableUID, UID};

use super::object::Object;

pub trait BasicTable {
    fn uid(&self) -> TableUID;
    fn get_object(&self, uid: UID) -> Option<&dyn Object>;
    fn get_object_mut(&mut self, uid: UID) -> Option<&mut dyn Object>;
    fn next_from(&self, uid: Option<UID>) -> Option<UID>;
}

pub struct Table<T: Object, const TABLE_UID: u64>(BTreeMap<ObjectUID<TABLE_UID>, T>);

impl<T: Object, const TABLE_UID: u64> Table<T, TABLE_UID> {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }
}

impl<T: Object, const TABLE_UID: u64> Deref for Table<T, TABLE_UID> {
    type Target = BTreeMap<ObjectUID<TABLE_UID>, T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Object, const TABLE_UID: u64> DerefMut for Table<T, TABLE_UID> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Object, const TABLE_UID: u64> BasicTable for Table<T, TABLE_UID> {
    fn uid(&self) -> TableUID {
        TableUID::new(TABLE_UID)
    }

    fn get_object(&self, uid: UID) -> Option<&dyn Object> {
        if let Ok(uid) = uid.try_into() {
            self.0.get(&uid).map(|object| object as &dyn Object)
        } else {
            None
        }
    }

    fn get_object_mut(&mut self, uid: UID) -> Option<&mut dyn Object> {
        if let Ok(uid) = uid.try_into() {
            self.0.get_mut(&uid).map(|object| object as &mut dyn Object)
        } else {
            None
        }
    }

    fn next_from(&self, uid: Option<UID>) -> Option<UID> {
        if let Some(Ok(uid)) = uid.map(|uid| ObjectUID::<TABLE_UID>::try_from(uid)) {
            let mut iter = self.0.range(uid..);
            if iter.next().is_none() {
                None
            } else {
                iter.next().map(|(k, _v)| k.clone().into())
            }
        } else {
            None
        }
    }
}
