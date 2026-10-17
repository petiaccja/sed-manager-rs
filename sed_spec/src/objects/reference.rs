//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_packet::{
    ObjectRef, Uid,
    token_stream::{Detokenize, Detokenizer, MessageError, Tokenize, Tokenizer},
};

use crate::{preconfig::core::shared::table_id, types::Type};

/// The UID of an object in the ACE table.
pub type AceRef = ObjectRef<{ table_id::ACE.to_u64() }>;

/// The UID of an object in the Authority table.
pub type AuthorityRef = ObjectRef<{ table_id::AUTHORITY.to_u64() }>;

/// The UID of an object in the CPIN table.
pub type ColumnRef = ObjectRef<{ table_id::COLUMN.to_u64() }>;

/// The UID of an object in the CPIN table.
pub type CPinRef = ObjectRef<{ table_id::C_PIN.to_u64() }>;

/// The UID of an object in the K_AES_128 table.
pub type KAes128Ref = ObjectRef<{ table_id::K_AES_128.to_u64() }>;

/// The UID of an object in the K_AES_256 table.
pub type KAes256Ref = ObjectRef<{ table_id::K_AES_256.to_u64() }>;

/// The UID of an object in the Locking table.
pub type LockingRangeRef = ObjectRef<{ table_id::LOCKING.to_u64() }>;

/// The UID of an object in the Locking table.
pub type LockingInfoRef = ObjectRef<{ table_id::LOCKING_INFO.to_u64() }>;

/// The UID of an object in the Locking table.
pub type LogListRef = ObjectRef<{ table_id::LOG_LIST.to_u64() }>;

/// The UID of an object in the MBRControl table.
pub type MbrControlRef = ObjectRef<{ table_id::MBR_CONTROL.to_u64() }>;

/// The UID of an object in the MethodID table.
pub type MethodRef = ObjectRef<{ table_id::METHOD_ID.to_u64() }>;

/// The UID of an object in the SP table.
pub type SecurityProviderRef = ObjectRef<{ table_id::SP.to_u64() }>;

/// The UID of an object in the Table table.
pub type TableDescRef = ObjectRef<{ table_id::TABLE.to_u64() }>;

/// The UID of an object in the Template table.
pub type TemplateRef = ObjectRef<{ table_id::TEMPLATE.to_u64() }>;

/// The UID of an object in the Table table.
pub type TypeRef = ObjectRef<{ table_id::TYPE.to_u64() }>;

/// A reference to an entry in the AccessControl meta-table.
///
/// This is slightly different than a regular reference, which is just an UID.
/// This is because the AccessControl table is not indexed by UIDs, it's indexed
/// by a pair of InvokingIDs and MethodIDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AccessControlRef {
    pub invoking_id: Uid,
    pub method_id: MethodRef,
}

/// A reference to any object that contains a credential.
///
/// Credential object have one field which is some sort of secret or password.
/// Examples include the C_PIN, K_AES_256, or C_RSA tables.
pub trait CredentialRef: Sized + TryFrom<Uid> + Into<Uid> + core::fmt::Debug + core::fmt::Display {}

impl CredentialRef for CPinRef {}
impl CredentialRef for KAes128Ref {}
impl CredentialRef for KAes256Ref {}
impl CredentialRef for MediaKeyRef {}

impl Type for AuthorityRef {
    const UID: TypeRef = TypeRef::new(0x0000_0005_0000_0C05);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MediaKeyRef {
    Aes128(KAes128Ref),
    Aes256(KAes256Ref),
}

impl From<MediaKeyRef> for Uid {
    fn from(value: MediaKeyRef) -> Self {
        match value {
            MediaKeyRef::Aes128(object_ref) => object_ref.into(),
            MediaKeyRef::Aes256(object_ref) => object_ref.into(),
        }
    }
}

impl From<KAes128Ref> for MediaKeyRef {
    fn from(value: KAes128Ref) -> Self {
        Self::Aes128(value)
    }
}

impl From<KAes256Ref> for MediaKeyRef {
    fn from(value: KAes256Ref) -> Self {
        Self::Aes256(value)
    }
}

impl TryFrom<Uid> for MediaKeyRef {
    type Error = Uid;

    fn try_from(value: Uid) -> Result<Self, Self::Error> {
        KAes128Ref::try_from(value)
            .map(Self::Aes128)
            .or_else(|_| KAes256Ref::try_from(value).map(Self::Aes256))
    }
}

impl core::fmt::Display for MediaKeyRef {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MediaKeyRef::Aes128(object_ref) => object_ref.fmt(f),
            MediaKeyRef::Aes256(object_ref) => object_ref.fmt(f),
        }
    }
}

impl Tokenize for MediaKeyRef {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        match self {
            MediaKeyRef::Aes128(object_ref) => object_ref.tokenize(tokenizer),
            MediaKeyRef::Aes256(object_ref) => object_ref.tokenize(tokenizer),
        }
    }
}

impl Detokenize for MediaKeyRef {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        Self::try_from(Uid::detokenize(detokenizer)?)
            .map_err(|uid| D::Error::message(format!("{uid} is not a media key")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use sed_packet::token_stream::{FromTokens, ToTokens};

    use crate::preconfig::opal_2::locking::{k_aes_128, k_aes_256, locking};

    #[rstest]
    // Null UIDs are assumed AES_128 because we cannot determine it.
    #[case::null(Uid::null(), Some(MediaKeyRef::Aes128(KAes128Ref::null())))]
    #[case::aes_128(k_aes_128::GLOBAL_RANGE_KEY.into(), Some(MediaKeyRef::Aes128(k_aes_128::GLOBAL_RANGE_KEY)))]
    #[case::aes_256(k_aes_256::GLOBAL_RANGE_KEY.into(), Some(MediaKeyRef::Aes256(k_aes_256::GLOBAL_RANGE_KEY)))]
    #[case::other_table(locking::GLOBAL_RANGE.into(), None)]
    fn media_key_ref_try_from_uid(#[case] uid: Uid, #[case] expected: Option<MediaKeyRef>) {
        assert_eq!(MediaKeyRef::try_from(uid).ok(), expected);
    }

    #[rstest]
    #[case::aes_128(k_aes_128::GLOBAL_RANGE_KEY.into())]
    #[case::aes_256(k_aes_256::GLOBAL_RANGE_KEY.into())]
    fn media_key_ref_tokenize_roundtrip(#[case] key: MediaKeyRef) {
        let tokens = key.to_tokens().unwrap();
        assert_eq!(tokens, Uid::from(key).to_tokens().unwrap());
        assert_eq!(MediaKeyRef::from_tokens(&tokens).unwrap(), key);
    }

    #[test]
    fn media_key_ref_detokenize_other_table() {
        let tokens = Uid::from(locking::GLOBAL_RANGE).to_tokens().unwrap();
        assert!(MediaKeyRef::from_tokens(&tokens).is_err());
    }
}
