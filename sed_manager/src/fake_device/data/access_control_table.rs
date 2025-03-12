use core::ops::{Deref, DerefMut};
use std::collections::BTreeMap;

use crate::messaging::uid::UID;
use crate::spec::basic_types::List;
use crate::spec::column_types::{ACERef, LogListRef, LogSelect, MethodRef, Name};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AccessControlRef {
    pub invoking_id: UID,
    pub method_id: MethodRef,
}

pub struct AccessControlEntry {
    pub common_name: Name,
    pub acl: List<ACERef>,
    pub log: LogSelect,
    pub add_ace_acl: List<ACERef>,
    pub remove_ace_acl: List<ACERef>,
    pub get_acl_acl: List<ACERef>,
    pub delete_method_acl: List<ACERef>,
    pub add_ace_log: LogSelect,
    pub remove_ace_log: LogSelect,
    pub get_acl_log: LogSelect,
    pub delete_method_log: LogSelect,
    pub log_to: LogListRef,
}

pub struct AccessControlTable(BTreeMap<AccessControlRef, AccessControlEntry>);

impl AccessControlRef {
    pub fn new(invoking_id: UID, method_id: MethodRef) -> Self {
        Self { invoking_id, method_id }
    }
}

impl AccessControlTable {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn get(&self, invoking_id: &UID, method_id: &MethodRef) -> Option<&AccessControlEntry> {
        self.0.get(&AccessControlRef { invoking_id: *invoking_id, method_id: *method_id })
    }

    pub fn get_mut(&mut self, invoking_id: &UID, method_id: &MethodRef) -> Option<&mut AccessControlEntry> {
        self.0.get_mut(&AccessControlRef { invoking_id: *invoking_id, method_id: *method_id })
    }
}

impl Deref for AccessControlTable {
    type Target = BTreeMap<AccessControlRef, AccessControlEntry>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AccessControlTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromIterator<(AccessControlRef, AccessControlEntry)> for AccessControlTable {
    fn from_iter<T: IntoIterator<Item = (AccessControlRef, AccessControlEntry)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl Default for AccessControlEntry {
    fn default() -> Self {
        Self {
            common_name: Name::default(),
            acl: vec![].into(),
            log: LogSelect::None,
            add_ace_acl: vec![].into(),
            remove_ace_acl: vec![].into(),
            get_acl_acl: vec![].into(),
            delete_method_acl: vec![].into(),
            add_ace_log: LogSelect::None,
            remove_ace_log: LogSelect::None,
            get_acl_log: LogSelect::None,
            delete_method_log: LogSelect::None,
            log_to: LogListRef::null(),
        }
    }
}
