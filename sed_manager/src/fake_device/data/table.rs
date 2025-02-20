use std::collections::BTreeMap;

use crate::messaging::uid::{ObjectUID, UID};
use crate::spec::basic_types::RestrictedObjectReference;

use super::object::Object;

pub trait BasicTable {
    fn uid(&self) -> UID;
    fn get_object(&self, uid: UID) -> Option<&dyn Object>;
    fn get_object_mut(&mut self, uid: UID) -> Option<&mut dyn Object>;
    fn next_from(&self, uid: Option<UID>) -> Option<UID>;
}

pub struct Table<T: Object, const TABLE_UID: u64>(pub BTreeMap<RestrictedObjectReference<TABLE_UID>, T>);

type ObjectRef<const TABLE_UID: u64> = RestrictedObjectReference<{ TABLE_UID }>;

impl<T: Object, const TABLE_UID: u64> Table<T, TABLE_UID> {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }
}

impl<T: Object, const TABLE_UID: u64> BasicTable for Table<T, TABLE_UID> {
    fn uid(&self) -> UID {
        UID::new(TABLE_UID)
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
