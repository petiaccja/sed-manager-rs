use crate::spec::basic_types::RestrictedObjectReference;
use crate::spec::table_id;

use super::define_column_type;

define_column_type!(AuthorityRef, 0x0000_0005_0000_0C05_u64, "Authority_object_ref");
define_column_type!(CredentialRef, 0x0000_0005_0000_0C0B_u64, "cred_object_uidref");
define_column_type!(LogListRef, 0x0000_0005_0000_0C0D_u64, "LogList_object_ref");

pub type AuthorityRef = RestrictedObjectReference<{ table_id::AUTHORITY.as_u64() }>;
pub type SPRef = RestrictedObjectReference<{ table_id::SP.as_u64() }>;
pub type CPinRef = RestrictedObjectReference<{ table_id::C_PIN.as_u64() }>;
pub type CredentialRef = CPinRef; // Should have more table_id but it's difficult to express without variadics.
pub type LogListRef = RestrictedObjectReference<{ table_id::LOG_LIST.as_u64() }>;
