use crate::messaging::uid::ObjectUID;
use crate::messaging::uid_range::ObjectUIDRange;
use crate::spec::table_id;

use super::define_column_type;

define_column_type!(AuthorityRef, 0x0000_0005_0000_0C05_u64, "Authority_object_ref");
define_column_type!(CredentialRef, 0x0000_0005_0000_0C0B_u64, "cred_object_uidref");
define_column_type!(LogListRef, 0x0000_0005_0000_0C0D_u64, "LogList_object_ref");

pub type AuthorityRef = ObjectUID<{ table_id::AUTHORITY.as_u64() }>;
pub type AuthorityRefRange = ObjectUIDRange<{ table_id::AUTHORITY.as_u64() }>;
pub type SPRef = ObjectUID<{ table_id::SP.as_u64() }>;
pub type SPRefRange = ObjectUIDRange<{ table_id::SP.as_u64() }>;
pub type CPINRef = ObjectUID<{ table_id::C_PIN.as_u64() }>;
pub type CPINRefRange = ObjectUIDRange<{ table_id::C_PIN.as_u64() }>;
pub type CredentialRef = CPINRef; // Should have more table_id but it's difficult to express without variadics.
pub type CredentialRefRange = CPINRefRange; // Should have more table_id but it's difficult to express without variadics.
pub type LogListRef = ObjectUID<{ table_id::LOG_LIST.as_u64() }>;
pub type MethodRef = ObjectUID<{ table_id::METHOD_ID.as_u64() }>;
