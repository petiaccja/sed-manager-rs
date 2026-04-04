use std::collections::HashSet;

use crate::objects::{KAes256Ref, LockingRangeRef};
use crate::preconfig::core::shared::table_id;
use crate::types::{
    AdvKeyMode, GeneralStatus, LastReencStatus, ReencryptRequest, ReencryptState, ResetType, VerifyMode,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[sed_spec_macros::object(table=table_id::LOCKING)]

pub struct LockingRange {
    pub uid: LockingRangeRef,
    pub name: String,
    pub common_name: String,
    pub range_start: u64,
    pub range_length: u64,
    pub read_lock_enabled: bool,
    pub write_lock_enabled: bool,
    pub read_locked: bool,
    pub write_locked: bool,
    pub lock_on_reset: HashSet<ResetType>,
    pub active_key: KAes256Ref,
    pub next_key: KAes256Ref,
    pub reencrypt_state: ReencryptState,
    pub reencrypt_request: ReencryptRequest,
    pub adv_key_mode: AdvKeyMode,
    pub verify_mode: VerifyMode,
    pub const_on_reset: HashSet<ResetType>,
    pub last_reencrypt_lba: u64,
    pub last_reenc_stat: LastReencStatus,
    pub general_status: GeneralStatus,
}
