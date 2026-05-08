use sed_packet::{ObjectRef, Uid};

use crate::{preconfig::core::shared::table_id, types::Type};

/// The UID of an object in the ACE table.
pub type AceRef = ObjectRef<{ table_id::ACE.to_u64() }>;

/// The UID of an object in the Authority table.
pub type AuthorityRef = ObjectRef<{ table_id::AUTHORITY.to_u64() }>;

/// The UID of an object in the CPIN table.
pub type ColumnRef = ObjectRef<{ table_id::COLUMN.to_u64() }>;

/// The UID of an object in the CPIN table.
pub type CPinRef = ObjectRef<{ table_id::C_PIN.to_u64() }>;

/// The UID of an object in the K_AES_256 table.
pub type KAes256Ref = ObjectRef<{ table_id::K_AES_256.to_u64() }>;

/// The UID of an object in the Locking table.
pub type LockingRangeRef = ObjectRef<{ table_id::LOCKING.to_u64() }>;

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
impl CredentialRef for KAes256Ref {}

impl Type for AuthorityRef {
    const UID: TypeRef = TypeRef::new(0x0000_0005_0000_0C05);
}
