use as_array::AsArray;

use crate::spec::column_types::{AuthorityRef, Date, LifeCycleState, MaxBytes32, Name, SPRef};

use super::cell::Cell;

#[derive(AsArray)]
#[as_array_traits(Cell)]
pub struct SP {
    pub uid: SPRef,
    pub name: Name,
    pub org: AuthorityRef,
    pub effective_auth: MaxBytes32,
    pub date_of_issue: Date,
    pub bytes: u64,
    pub life_cycle_state: LifeCycleState,
    pub frozen: bool,
}

impl SP {
    pub const UID: u16 = 0;
    pub const NAME: u16 = 1;
    pub const ORG: u16 = 2;
    pub const EFFECTIVE_AUTH: u16 = 3;
    pub const DATE_OF_ISSUE: u16 = 4;
    pub const BYTES: u16 = 5;
    pub const LIFE_CYCLE_STATE: u16 = 6;
    pub const FROZEN: u16 = 7;
}

impl Default for SP {
    fn default() -> Self {
        Self {
            uid: SPRef::null(),
            name: Name::default(),
            org: AuthorityRef::null(),
            effective_auth: MaxBytes32::default(),
            date_of_issue: Date::default(),
            bytes: 0,
            life_cycle_state: LifeCycleState::Issued,
            frozen: false,
        }
    }
}
