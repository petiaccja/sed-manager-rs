//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::messaging::uid::ObjectUID;
use crate::messaging::uid_range::ObjectUIDRange;
use crate::spec::table_id::*;

use super::define_column_type;

define_column_type!(AuthorityRef, 0x0000_0005_0000_0C05_u64, "Authority_object_ref");
define_column_type!(CredentialRef, 0x0000_0005_0000_0C0B_u64, "cred_object_uidref");
define_column_type!(LogListRef, 0x0000_0005_0000_0C0D_u64, "LogList_object_ref");

pub type ACERef = ObjectUID<{ ACE.mask() }>;
pub type AuthorityRef = ObjectUID<{ AUTHORITY.mask() }>;
pub type AuthorityRefRange = ObjectUIDRange<{ AUTHORITY.mask() }>;
pub type SPRef = ObjectUID<{ SP.mask() }>;
pub type SPRefRange = ObjectUIDRange<{ SP.mask() }>;
pub type CPINRef = ObjectUID<{ C_PIN.mask() }>;
pub type CPINRefRange = ObjectUIDRange<{ C_PIN.mask() }>;
pub type CredentialRefRange = CPINRefRange;
pub type LogListRef = ObjectUID<{ LOG_LIST.mask() }>;
pub type LockingRangeRef = ObjectUID<{ LOCKING.mask() }>;
pub type MediaKeyRef = ObjectUID<{ K_AES_128.mask() | K_AES_256.mask() }>;
pub type KAES256Ref = ObjectUID<{ K_AES_256.mask() }>;
pub type MethodRef = ObjectUID<{ METHOD_ID.mask() }>;
pub type MBRControlRef = ObjectUID<{ MBR_CONTROL.mask() }>;
pub type TableDescRef = ObjectUID<{ TABLE.mask() }>;
pub type TemplateRef = ObjectUID<{ TEMPLATE.mask() }>;
pub type ColumnRef = ObjectUID<{ COLUMN.mask() }>;

/// UIDs for any of the C_* tables.
///
/// Not all tables are currently present in the mask, but they could be added.
/// I've also added the K_AES_* tables despite the Core Spec not mentioning them.
/// This is because GenKey is callable on a CredentialObjectUID, but GenKey is
/// also callable on the K_AES_* objects. Go figure?
pub type CredentialRef = ObjectUID<
    {
        C_PIN.mask() | C_AES_128.mask() | C_AES_256.mask() | K_AES_128.mask() | K_AES_256.mask()
        /* More tables... */
    },
>;
