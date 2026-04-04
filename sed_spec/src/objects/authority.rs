use sed_packet::Uid;

use crate::objects::{AuthorityRef, LogListRef};
use crate::preconfig::core::shared::table_id;
use crate::types::{AuthMethod, Date, HashProtocol, LogSelect, MessagingType};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[sed_spec_macros::object(table=table_id::AUTHORITY)]
pub struct Authority {
    pub uid: AuthorityRef,
    pub name: String,
    pub common_name: String,
    pub is_class: bool,
    pub class: AuthorityRef,
    pub enabled: bool,
    pub secure: MessagingType,
    pub hash_and_sign: HashProtocol,
    pub present_certificate: bool,
    pub operation: AuthMethod,
    pub credential: Uid,
    pub response_sign: AuthorityRef,
    pub response_exch: AuthorityRef,
    pub clock_start: Date,
    pub clock_end: Date,
    pub limit: u32,
    pub uses: u32,
    pub log: LogSelect,
    pub log_to: LogListRef,
}
