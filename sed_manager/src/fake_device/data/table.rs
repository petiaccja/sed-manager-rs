use std::collections::BTreeMap;

use crate::messaging::uid::UID;

use super::object::Object;

pub trait Table {
    fn get(&self, object: UID) -> Option<&dyn Object>;
    fn get_mut(&mut self, object: UID) -> Option<&mut dyn Object>;
    fn next(&self, object: UID) -> UID;
}

pub struct TableData<T: Object>(BTreeMap<UID, T>);

impl<T: Object> Table for TableData<T> {
    fn get(&self, object: UID) -> Option<&dyn Object> {
        self.0.get(&object).map(|v| v as &dyn Object)
    }

    fn get_mut(&mut self, object: UID) -> Option<&mut dyn Object> {
        self.0.get_mut(&object).map(|v| v as &mut dyn Object)
    }

    fn next(&self, object: UID) -> UID {
        let mut next_range = self.0.range(object..);
        let Some((_, _)) = next_range.next() else {
            return UID::null();
        };
        let Some((next_uid, _)) = next_range.next() else { return UID::null() };
        next_uid.clone()
    }
}
