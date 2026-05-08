use sed_packet::{Object, TableRef};

use crate::{
    objects::{AccessControlRef, AceRef, LogListRef},
    preconfig::core::shared::table_id,
    types::LogSelect,
};

#[derive(Debug)]
pub struct AccessControl {
    pub common_name: String,
    pub acl: Vec<AceRef>,
    pub log: LogSelect,
    pub add_ace_acl: Vec<AceRef>,
    pub remove_ace_acl: Vec<AceRef>,
    pub get_acl_acl: Vec<AceRef>,
    pub delete_method_acl: Vec<AceRef>,
    pub add_ace_log: LogSelect,
    pub remove_ace_log: LogSelect,
    pub get_acl_log: LogSelect,
    pub delete_method_log: LogSelect,
    pub log_to: LogListRef,
}

impl Default for AccessControl {
    fn default() -> Self {
        Self {
            common_name: String::new(),
            acl: Vec::new(),
            log: LogSelect::None,
            add_ace_acl: Vec::new(),
            remove_ace_acl: Vec::new(),
            get_acl_acl: Vec::new(),
            delete_method_acl: Vec::new(),
            add_ace_log: LogSelect::None,
            remove_ace_log: LogSelect::None,
            get_acl_log: LogSelect::None,
            delete_method_log: LogSelect::None,
            log_to: LogListRef::new(0x00000a0200000001_u64),
        }
    }
}

impl Object for AccessControl {
    const TABLE: TableRef = table_id::ACCESS_CONTROL;
    type Ref = AccessControlRef;
    const FIELD_COUNT: u16 = 12;

    fn active_fields(&self) -> Vec<u16> {
        (0..Self::FIELD_COUNT).collect()
    }

    fn update(&mut self, other: Self) {
        *self = other;
    }
}
