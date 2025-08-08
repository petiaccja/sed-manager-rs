/// The God authority can perform any action and modify any table without password authentication.
///
/// This is useful for testing to bypass access control.
use crate::fake_device::data::access_control_table::{AccessControlEntry, AccessControlRef, AccessControlTable};
use crate::fake_device::data::object_table::{ACETable, AuthorityTable};
use crate::messaging::uid::TableUID;
use crate::spec::column_types::{ACERef, AuthMethod, AuthorityRef, MethodRef};
use crate::spec::objects::ace::ace_expr;
use crate::spec::objects::{Authority, ACE};
use crate::spec::{method_id, table_id};

/// A special authority that has permissions for everything and needs no login.
pub const AUTHORITY_GOD: AuthorityRef = AuthorityRef::new(table_id::AUTHORITY.as_u64() + 0xFFF_FFF0);

/// A special ACE that grants access to the god authority modify any table.
pub const ACE_GOD: ACERef = ACERef::new(table_id::ACE.as_u64() + 0xFFF_FFF0);

pub fn append_god_authority(mut authority_table: AuthorityTable) -> AuthorityTable {
    authority_table.insert(
        AUTHORITY_GOD,
        Authority {
            uid: AUTHORITY_GOD,
            name: "God".into(),
            is_class: false,
            operation: AuthMethod::None,
            ..Default::default()
        },
    );
    authority_table
}

pub fn append_god_ace(mut ace_table: ACETable) -> ACETable {
    ace_table.insert(
        ACE_GOD,
        ACE {
            uid: ACE_GOD,
            boolean_expr: ace_expr!((AUTHORITY_GOD)),
            columns: [].into_iter().collect(),
            ..Default::default()
        },
    );
    ace_table
}

pub fn append_god_access_control(mut access_control: AccessControlTable) -> AccessControlTable {
    const TABLES: &[TableUID] = &[
        table_id::TABLE,
        table_id::SP_INFO,
        table_id::SP_TEMPLATES,
        table_id::COLUMN,
        table_id::TYPE,
        table_id::ACCESS_CONTROL,
        table_id::ACE,
        table_id::AUTHORITY,
        table_id::CERTIFICATES,
        table_id::METHOD_ID,
        table_id::C_PIN,
        table_id::C_RSA_1024,
        table_id::C_RSA_2048,
        table_id::C_AES_128,
        table_id::C_AES_256,
        table_id::C_EC_160,
        table_id::C_EC_192,
        table_id::C_EC_224,
        table_id::C_EC_256,
        table_id::C_EC_384,
        table_id::C_EC_521,
        table_id::C_EC_163,
        table_id::C_EC_233,
        table_id::C_EC_283,
        table_id::C_HMAC_160,
        table_id::C_HMAC_256,
        table_id::C_HMAC_384,
        table_id::C_HMAC_512,
        table_id::SECRET_PROTECT,
        table_id::T_PER_INFO,
        table_id::CRYPTO_SUITE,
        table_id::TEMPLATE,
        table_id::SP,
        table_id::CLOCK_TIME,
        table_id::H_SHA_1,
        table_id::H_SHA_256,
        table_id::H_SHA_384,
        table_id::H_SHA_512,
        table_id::LOG,
        table_id::LOG_LIST,
        table_id::LOCKING_INFO,
        table_id::LOCKING,
        table_id::MBR_CONTROL,
        table_id::MBR,
        table_id::K_AES_128,
        table_id::K_AES_256,
    ];
    const METHODS: &[MethodRef] = &[
        method_id::DELETE_SP,
        method_id::CREATE_TABLE,
        method_id::DELETE,
        method_id::CREATE_ROW,
        method_id::DELETE_ROW,
        method_id::OBSOLETE_0006,
        method_id::OBSOLETE_0007,
        method_id::NEXT,
        method_id::GET_FREE_SPACE,
        method_id::GET_FREE_ROWS,
        method_id::DELETE_METHOD,
        method_id::OBSOLETE,
        method_id::GET_ACL,
        method_id::ADD_ACE,
        method_id::REMOVE_ACE,
        method_id::GEN_KEY,
        method_id::RESERVED_0011,
        method_id::GET_PACKAGE,
        method_id::SET_PACKAGE,
        method_id::GET,
        method_id::SET,
        method_id::AUTHENTICATE,
        method_id::ISSUE_SP,
        method_id::RESERVED_0202,
        method_id::RESERVED_0203,
        method_id::GET_CLOCK,
        method_id::RESET_CLOCK,
        method_id::SET_CLOCK_HIGH,
        method_id::SET_LAG_HIGH,
        method_id::SET_CLOCK_LOW,
        method_id::SET_LAG_LOW,
        method_id::INCREMENT_COUNTER,
        method_id::RANDOM,
        method_id::SALT,
        method_id::DECRYPT_INIT,
        method_id::DECRYPT,
        method_id::DECRYPT_FINALIZE,
        method_id::ENCRYPT_INIT,
        method_id::ENCRYPT,
        method_id::ENCRYPT_FINALIZE,
        method_id::HMAC_INIT,
        method_id::HMAC,
        method_id::HMAC_FINALIZE,
        method_id::HASH_INIT,
        method_id::HASH,
        method_id::HASH_FINALIZE,
        method_id::SIGN,
        method_id::VERIFY,
        method_id::XOR,
        method_id::ADD_LOG,
        method_id::CREATE_LOG,
        method_id::CLEAR_LOG,
        method_id::FLUSH_LOG,
        method_id::RESERVED_0803,
        method_id::REVERT,
        method_id::REVERT_SP,
        method_id::ACTIVATE,
        method_id::REACTIVATE,
        method_id::ERASE,
    ];
    for method in METHODS {
        for table in TABLES {
            if let Some(acl) = access_control.get_mut(&table.as_uid(), method) {
                acl.acl.push(ACE_GOD);
            } else {
                access_control.insert(
                    AccessControlRef { invoking_id: table.as_uid(), method_id: *method },
                    AccessControlEntry { acl: vec![ACE_GOD].into(), ..Default::default() },
                );
            }
        }
    }
    access_control
}
