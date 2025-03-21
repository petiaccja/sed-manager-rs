//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use as_array::AsArray;

use crate::spec::column_types::{
    AuthMethod, AuthorityRef, CredentialRef, Date, HashProtocol, LogListRef, LogSelect, MessagingType, Name,
};

use super::cell::Cell;

#[derive(AsArray)]
#[as_array_traits(Cell)]
pub struct Authority {
    pub uid: AuthorityRef,
    pub name: Name,
    pub common_name: Name,
    pub is_class: bool,
    pub class: AuthorityRef,
    pub enabled: bool,
    pub secure: MessagingType,
    pub hash_and_sign: HashProtocol,
    pub present_certificate: bool,
    pub operation: AuthMethod,
    pub credential: CredentialRef,
    pub response_sign: AuthorityRef,
    pub response_exch: AuthorityRef,
    pub clock_start: Date,
    pub clock_end: Date,
    pub limit: u32,
    pub uses: u32,
    pub log: LogSelect,
    pub log_to: LogListRef,
}

impl Authority {
    pub const UID: u16 = 0;
    pub const NAME: u16 = 1;
    pub const COMMON_NAME: u16 = 2;
    pub const IS_CLASS: u16 = 3;
    pub const CLASS: u16 = 4;
    pub const ENABLED: u16 = 5;
    pub const SECURE: u16 = 6;
    pub const HASH_AND_SIGN: u16 = 7;
    pub const PRESENT_CERTIFICATE: u16 = 8;
    pub const OPERATION: u16 = 9;
    pub const CREDENTIAL: u16 = 10;
    pub const RESPONSE_SIGN: u16 = 11;
    pub const RESPONSE_EXCH: u16 = 12;
    pub const CLOCK_START: u16 = 13;
    pub const CLOCK_END: u16 = 14;
    pub const LIMIT: u16 = 15;
    pub const USES: u16 = 16;
    pub const LOG: u16 = 17;
    pub const LOG_TO: u16 = 18;
}

impl Default for Authority {
    fn default() -> Self {
        Self {
            uid: AuthorityRef::null(),
            name: Name::default(),
            common_name: Name::default(),
            is_class: true,
            class: AuthorityRef::null(),
            enabled: true,
            secure: MessagingType::None,
            hash_and_sign: HashProtocol::None,
            present_certificate: false,
            operation: AuthMethod::None,
            credential: CredentialRef::null(),
            response_sign: AuthorityRef::null(),
            response_exch: AuthorityRef::null(),
            clock_start: Date::default(),
            clock_end: Date::default(),
            limit: 0,
            uses: 0,
            log: LogSelect::None,
            log_to: LogListRef::null(),
        }
    }
}
