use sed_packet::ObjectRef;

use crate::{preconfig::core::shared::table_id, types::Type};

/// The UID of an object in the ACE table.
pub type AceRef = ObjectRef<{ table_id::ACE.to_u64() }>;

/// The UID of an object in the Authority table.
pub type AuthorityRef = ObjectRef<{ table_id::AUTHORITY.to_u64() }>;

/// The UID of an object in the CPIN table.
pub type CPinRef = ObjectRef<{ table_id::C_PIN.to_u64() }>;

/// The UID of an object in the K_AES_256 table.
pub type KAes256Ref = ObjectRef<{ table_id::K_AES_256.to_u64() }>;

/// The UID of an object in the Locking table.
pub type LockingRangeRef = ObjectRef<{ table_id::LOCKING.to_u64() }>;

/// The UID of an object in the MBRControl table.
pub type MbrControlRef = ObjectRef<{ table_id::MBR_CONTROL.to_u64() }>;

/// The UID of an object in the SP table.
pub type SpRef = ObjectRef<{ table_id::SP.to_u64() }>;

/// The UID of an object in the Table table.
pub type TableRef = ObjectRef<{ table_id::TABLE.to_u64() }>;

/// The UID of an object in the Table table.
pub type TypeRef = ObjectRef<{ table_id::TYPE.to_u64() }>;

impl Type for AuthorityRef {
    const UID: TypeRef = TypeRef::new_unchecked(0x0000_0005_0000_0C05);
}
