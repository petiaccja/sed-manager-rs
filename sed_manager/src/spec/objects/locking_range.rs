use as_array::AsArray;

use crate::spec::column_types::{
    AdvKeyMode, GeneralStatus, LastReencStatus, LockingRangeRef, MediaKeyRef, Name, ReencryptRequest, ReencryptState,
    ResetTypes, VerifyMode,
};

use super::cell::Cell;

#[derive(AsArray)]
#[as_array_traits(Cell)]
pub struct LockingRange {
    pub uid: LockingRangeRef,
    pub name: Name,
    pub common_name: Name,
    pub range_start: u64,
    pub range_length: u64,
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
    pub const UID: u16 = 0;
    pub const NAME: u16 = 1;
    pub const COMMON_NAME: u16 = 2;
    pub const RANGE_START: u16 = 3;
    pub const RANGE_LENGTH: u16 = 4;
    pub const READ_LOCK_ENABLED: u16 = 5;
    pub const WRITE_LOCK_ENABLED: u16 = 6;
    pub const READ_LOCKED: u16 = 7;
    pub const WRITE_LOCKED: u16 = 8;
    pub const LOCK_ON_RESET: u16 = 9;
    pub const ACTIVE_KEY: u16 = 10;
    pub const NEXT_KEY: u16 = 11;
    pub const REENCRYPT_STATE: u16 = 12;
    pub const REENCRYPT_REQUEST: u16 = 13;
    pub const ADV_KEY_MODE: u16 = 14;
    pub const VERIFY_MODE: u16 = 15;
    pub const CONST_ON_RESET: u16 = 16;
    pub const LAST_REENCRYPT_LBA: u16 = 17;
    pub const LAST_REENC_STAT: u16 = 18;
    pub const GENERAL_STATUS: u16 = 19;
}

impl Default for LockingRange {
    fn default() -> Self {
        Self {
            uid: LockingRangeRef::null(),
            name: Name::default(),
            common_name: Name::default(),
            range_start: 0,
            range_length: 0,
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
