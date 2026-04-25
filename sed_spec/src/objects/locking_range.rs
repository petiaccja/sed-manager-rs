use std::collections::HashSet;

use sed_spec_macros::{DetokenizeStruct, FieldList, Object, TokenizeStruct};

use crate::objects::{KAes256Ref, LockingRangeRef};
use crate::preconfig::core::shared::table_id;
use crate::types::{
    AdvKeyMode, GeneralStatus, LastReencStatus, ReencryptRequest, ReencryptState, ResetType, VerifyMode,
};

#[derive(Debug, Clone, Default, PartialEq, Eq, Object, TokenizeStruct, DetokenizeStruct, FieldList)]
#[object(table=table_id::LOCKING)]
pub struct LockingRange {
    pub uid: Option<LockingRangeRef>,
    pub name: Option<String>,
    pub common_name: Option<String>,
    pub range_start: Option<u64>,
    pub range_length: Option<u64>,
    pub read_lock_enabled: Option<bool>,
    pub write_lock_enabled: Option<bool>,
    pub read_locked: Option<bool>,
    pub write_locked: Option<bool>,
    pub lock_on_reset: Option<HashSet<ResetType>>,
    pub active_key: Option<KAes256Ref>,
    pub next_key: Option<KAes256Ref>,
    pub reencrypt_state: Option<ReencryptState>,
    pub reencrypt_request: Option<ReencryptRequest>,
    pub adv_key_mode: Option<AdvKeyMode>,
    pub verify_mode: Option<VerifyMode>,
    pub const_on_reset: Option<HashSet<ResetType>>,
    pub last_reencrypt_lba: Option<u64>,
    pub last_reenc_stat: Option<LastReencStatus>,
    pub general_status: Option<GeneralStatus>,
}
