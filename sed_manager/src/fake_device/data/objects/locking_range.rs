use as_array::AsArray;

use crate::messaging::uid::UID;
use crate::messaging::value::Value;
use crate::spec::column_types::{
    AdvKeyMode, GeneralStatus, LastReencStatus, LockingRangeRef, MediaKeyRef, Name, ReencryptRequest, ReencryptState,
    ResetTypes, VerifyMode,
};

use super::super::field::Field;
use super::super::object::GenericObject;

#[derive(AsArray)]
#[as_array_traits(Field)]
pub struct LockingRange {
    pub uid: LockingRangeRef,
    pub name: Option<Name>,
    pub common_name: Option<Name>,
    pub range_start: u64,
    pub range_end: u64,
    pub read_lock_enabled: bool,
    pub write_lock_enabled: bool,
    pub read_locked: bool,
    pub write_locked: bool,
    pub lock_on_reset: ResetTypes,
    pub active_key: MediaKeyRef,
    pub next_key: MediaKeyRef,
    pub reencrypt_state: ReencryptState,
    pub reencrypt_request: ReencryptRequest,
    pub adv_key_mode: AdvKeyMode,
    pub verify_mode: VerifyMode,
    pub const_on_reset: ResetTypes,
    pub last_reencrypt_lba: u64,
    pub last_reenc_stat: LastReencStatus,
    pub general_status: GeneralStatus,
}

impl LockingRange {
    pub fn new(uid: LockingRangeRef) -> Self {
        Self {
            uid,
            name: None,
            common_name: None,
            range_start: 0,
            range_end: 0,
            read_lock_enabled: false,
            write_lock_enabled: false,
            read_locked: false,
            write_locked: false,
            lock_on_reset: ResetTypes::PowerCycle,
            active_key: MediaKeyRef::null(),
            next_key: MediaKeyRef::null(),
            reencrypt_state: ReencryptState::Idle,
            reencrypt_request: ReencryptRequest::Unknown,
            adv_key_mode: AdvKeyMode::AutoAdvanceKeys,
            verify_mode: VerifyMode::NoVerify,
            const_on_reset: ResetTypes::PowerCycle,
            last_reencrypt_lba: 0,
            last_reenc_stat: LastReencStatus::Success,
            general_status: GeneralStatus::None,
        }
    }
}

impl GenericObject for LockingRange {
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
