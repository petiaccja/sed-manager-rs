use std::collections::BTreeMap;

use crate::messaging::uid::UID;
use crate::spec::basic_types::{ObjectReference, RestrictedObjectReference};

use super::object::Object;

pub trait BasicTable {
    fn uid(&self) -> UID;
    fn get_object(&self, uid: ObjectReference) -> Option<&dyn Object>;
    fn get_object_mut(&mut self, uid: ObjectReference) -> Option<&mut dyn Object>;
    fn next_from(&self, uid: Option<ObjectReference>) -> Option<ObjectReference>;
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

    fn get_object(&self, uid: ObjectReference) -> Option<&dyn Object> {
        self.0.get(&uid.0.into()).map(|object| object as &dyn Object)
    }

    fn get_object_mut(&mut self, uid: ObjectReference) -> Option<&mut dyn Object> {
        self.0.get_mut(&uid.0.into()).map(|object| object as &mut dyn Object)
    }

    fn next_from(&self, uid: Option<ObjectReference>) -> Option<ObjectReference> {
        let range = uid.map(|uid| ObjectRef::<TABLE_UID>::from(uid.0)..);
        let mut iter = range.map(|range| self.0.range(range)).unwrap_or(self.0.range(..));
        if iter.next().is_none() {
            None
        } else {
            iter.next().map(|(k, _v)| k.clone().0.into())
        }
    }
}
