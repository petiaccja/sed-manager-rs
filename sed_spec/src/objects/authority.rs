use sed_packet::Uid;
use sed_spec_macros::{DetokenizeStruct, Object, TokenizeStruct, FieldList};

use crate::objects::{AuthorityRef, LogListRef};
use crate::preconfig::core::shared::table_id;
use crate::types::{AuthMethod, Date, HashProtocol, LogSelect, MessagingType};

#[derive(Debug, Clone, Default, PartialEq, Eq, Object, TokenizeStruct, DetokenizeStruct, FieldList)]
#[object(table=table_id::AUTHORITY)]
pub struct Authority {
    pub uid: Option<AuthorityRef>,
    pub name: Option<String>,
    pub common_name: Option<String>,
    pub is_class: Option<bool>,
    pub class: Option<AuthorityRef>,
    pub enabled: Option<bool>,
    pub secure: Option<MessagingType>,
    pub hash_and_sign: Option<HashProtocol>,
    pub present_certificate: Option<bool>,
    pub operation: Option<AuthMethod>,
    pub credential: Option<Uid>,
    pub response_sign: Option<AuthorityRef>,
    pub response_exch: Option<AuthorityRef>,
    pub clock_start: Option<Date>,
    pub clock_end: Option<Date>,
    pub limit: Option<u32>,
    pub uses: Option<u32>,
    pub log: Option<LogSelect>,
    pub log_to: Option<LogListRef>,
}
