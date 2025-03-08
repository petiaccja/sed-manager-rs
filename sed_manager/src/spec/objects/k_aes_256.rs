use as_array::AsArray;

use crate::spec::column_types::{KAES256Ref, Key256, Name, SymmetricModeMedia};

use super::cell::Cell;

#[derive(AsArray)]
#[as_array_traits(Cell)]
pub struct KAES256 {
    pub uid: KAES256Ref,
    pub name: Name,
    pub common_name: Name,
    pub key: Key256,
    pub mode: SymmetricModeMedia,
}

impl KAES256 {
    pub const UID: u16 = 0;
    pub const NAME: u16 = 1;
    pub const COMMON_NAME: u16 = 2;
    pub const KEY: u16 = 3;
    pub const MODE: u16 = 4;
}

impl Default for KAES256 {
    fn default() -> Self {
        Self {
            uid: KAES256Ref::null(),
            name: Name::default(),
            common_name: Name::default(),
            key: Key256::Bytes64([0; 64]),
            mode: SymmetricModeMedia::XTS,
        }
    }
}
