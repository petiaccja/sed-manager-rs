use as_array::AsArray;

use crate::messaging::uid::TableUID;
use crate::spec::basic_types::ByteTableReference;
use crate::spec::column_types::{CPINRef, Name, Password};

use super::cell::Cell;

#[derive(AsArray)]
#[as_array_traits(Cell)]
pub struct CPIN {
    pub uid: CPINRef,
    pub name: Name,
    pub common_name: Name,
    pub pin: Password,
    pub char_set: ByteTableReference,
    pub try_limit: u32,
    pub tries: u32,
    pub persistence: bool,
}

impl CPIN {
    pub const UID: u16 = 0;
    pub const NAME: u16 = 1;
    pub const COMMON_NAME: u16 = 2;
    pub const PIN: u16 = 3;
    pub const CHAR_SET: u16 = 4;
    pub const TRY_LIMIT: u16 = 5;
    pub const TRIES: u16 = 6;
    pub const PERSISTENCE: u16 = 7;
}

impl Default for CPIN {
    fn default() -> Self {
        Self {
            uid: CPINRef::null(),
            name: Name::default(),
            common_name: Name::default(),
            pin: Password::default(),
            char_set: ByteTableReference(TableUID::null()),
            try_limit: 0,
            tries: 0,
            persistence: false,
        }
    }
}
