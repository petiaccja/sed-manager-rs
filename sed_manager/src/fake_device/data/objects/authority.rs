use as_array::AsArray;

use crate::messaging::uid::UID;
use crate::messaging::value::Value;
use crate::spec::column_types::{
    AuthMethod, AuthorityRef, CredentialRef, Date, HashProtocol, LogListRef, LogSelect, MessagingType, Name,
};

use super::super::{Field, Object};

#[derive(AsArray)]
#[as_array_traits(Field)]
pub struct Authority {
    pub uid: AuthorityRef,
    pub name: Option<Name>,
    pub common_name: Option<Name>,
    pub is_class: Option<bool>,
    pub class: Option<AuthorityRef>,
    pub enabled: Option<bool>,
    pub secure: Option<MessagingType>,
    pub hash_and_sign: Option<HashProtocol>,
    pub present_certificate: Option<bool>,
    pub operation: Option<AuthMethod>,
    pub credential: Option<CredentialRef>,
    pub response_sign: Option<AuthorityRef>,
    pub response_exch: Option<AuthorityRef>,
    pub clock_start: Option<Date>,
    pub clock_end: Option<Date>,
    pub limit: Option<u32>,
    pub uses: Option<u32>,
    pub log: Option<LogSelect>,
    pub log_to: Option<LogListRef>,
}

impl Authority {
    pub fn new(uid: AuthorityRef) -> Self {
        Self {
            uid,
            name: None,
            common_name: None,
            is_class: None,
            class: None,
            enabled: None,
            secure: None,
            hash_and_sign: None,
            present_certificate: None,
            operation: None,
            credential: None,
            response_sign: None,
            response_exch: None,
            clock_start: None,
            clock_end: None,
            limit: None,
            uses: None,
            log: None,
            log_to: None,
        }
    }
}

impl Object for Authority {
    fn uid(&self) -> UID {
        self.uid.into()
    }

    fn len(&self) -> usize {
        self.as_array().len()
    }

    fn is_column_empty(&self, column: usize) -> bool {
        self.as_array()[column].is_empty()
    }

    fn get_column(&self, column: usize) -> crate::messaging::value::Value {
        self.as_array()[column].to_value()
    }

    fn try_set_column(&mut self, column: usize, value: Value) -> Result<(), Value> {
        self.as_array_mut()[column].try_replace_with_value(value)
    }
}
