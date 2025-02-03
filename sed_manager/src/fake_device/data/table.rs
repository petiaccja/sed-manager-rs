use std::collections::BTreeMap;

use crate::messaging::{types::RestrictedObjectReference, uid::UID};

use super::object::Object;

pub trait BasicTable {
    fn uid(&self) -> UID;

    fn get_object(&self, uid: UID) -> Option<&dyn Object>;

    fn get_object_mut(&mut self, uid: UID) -> Option<&mut dyn Object>;
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
        self.0.get(&uid.into()).map(|object| object as &dyn Object)
    }

    fn get_object_mut(&mut self, uid: UID) -> Option<&mut dyn Object> {
        self.0.get_mut(&uid.into()).map(|object| object as &mut dyn Object)
    }
}
