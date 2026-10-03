//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_packet::ObjectUid;
use sed_spec_macros::{DetokenizeStruct, FieldList, Object, TokenizeField, TokenizeStruct};

use crate::objects::LockingInfoRef;
use crate::preconfig::core::shared::table_id;
use crate::types::{EncryptionSupport, KeysAvailableCondition};

#[derive(Debug, Clone, Default, PartialEq, Eq, Object, TokenizeStruct, DetokenizeStruct, FieldList, TokenizeField)]
#[object(table=table_id::LOCKING_INFO)]
pub struct LockingInfo {
    pub uid: Option<LockingInfoRef>,
    pub name: Option<String>,
    pub version: Option<u32>,
    pub encryption_support: Option<EncryptionSupport>,
    pub max_ranges: Option<u32>,
    pub max_re_encryptions: Option<u32>,
    pub keys_available_cfg: Option<KeysAvailableCondition>,
}

impl ObjectUid for LockingInfo {
    fn uid(&self) -> Option<Self::Ref> {
        self.uid
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Object, TokenizeStruct, DetokenizeStruct, FieldList, TokenizeField)]
#[object(table=table_id::LOCKING_INFO)]
pub struct LockingInfoExt {
    pub uid: Option<LockingInfoRef>,
    pub name: Option<String>,
    pub version: Option<u32>,
    pub encryption_support: Option<EncryptionSupport>,
    pub max_ranges: Option<u32>,
    pub max_re_encryptions: Option<u32>,
    pub keys_available_cfg: Option<KeysAvailableCondition>,
    // The alignment fields below are available only for Opal 2, Opalite, Pyrite, and Ruby.
    pub alignmnet_required: Option<bool>,
    pub logical_block_size: Option<u32>,
    pub alignment_granularity: Option<u64>,
    pub lowest_aligned_lba: Option<u64>,
}

impl ObjectUid for LockingInfoExt {
    fn uid(&self) -> Option<Self::Ref> {
        self.uid
    }
}
