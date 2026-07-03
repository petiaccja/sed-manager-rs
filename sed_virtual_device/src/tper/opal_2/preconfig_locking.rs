use std::ops::Range;

use sed_spec::{
    ace_expr,
    objects::{
        AccessControl, AccessControlRef, Ace, Authority, AuthorityRef, CPin, LockingRange, MbrControl, TableDesc,
    },
    preconfig::{
        core::shared::{invoking_id::THIS_SP, mbr_control, table},
        opal_2::{
            admin::sp,
            locking::{ace, authority, c_pin, k_aes_256, locking},
        },
    },
    types::{AuthMethod, TableKind},
};

use crate::tper::{
    Locking,
    preconfig_shared::{AllColumns, IntoTable},
    security_provider::Table,
};
use sed_spec::objects::KAes256;

const ADMINS: Range<usize> = 0..4;
const USERS: Range<usize> = 0..8;
const RANGES: Range<usize> = 0..8;
const MBR_SIZE: u32 = 0x08000000;
const DATA_STORE_SIZE: u32 = 0x00A00000;

pub fn preconfig() -> Locking {
    Locking {
        uid: sp::LOCKING,
        access_control: access_control(),
        ace: ace(),
        authority: authority(),
        c_pin: c_pin(),
        k_aes_256: k_aes_256(),
        locking: locking(),
        mbr_control: mbr_control(),
        table: table(),
        mbr: vec![0u8; MBR_SIZE as usize],
        data_store: vec![vec![0u8; DATA_STORE_SIZE as usize]],
    }
}

pub fn access_control() -> Table<AccessControl> {
    use sed_spec::preconfig::core::shared::{method_id::*, table_id::*};

    let fixed = [
        // SP
        (
            AccessControlRef { invoking_id: THIS_SP.into(), method_id: RANDOM },
            AccessControl { acl: vec![ace::ANYBODY], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: THIS_SP.into(), method_id: REVERT_SP },
            AccessControl { acl: vec![ace::ADMIN], ..Default::default() },
        ),
        // Table
        (
            AccessControlRef { invoking_id: TABLE.into(), method_id: NEXT },
            AccessControl { acl: vec![ace::ANYBODY], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: TABLE.into(), method_id: GET },
            AccessControl { acl: vec![ace::ANYBODY], ..Default::default() },
        ),
        // ACE
        (
            AccessControlRef { invoking_id: ACE.into(), method_id: NEXT },
            AccessControl { acl: vec![ace::ANYBODY], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: ACE.into(), method_id: GET },
            AccessControl { acl: vec![ace::ACE_GET_ALL], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: ace::ACE_GET_ALL.into(), method_id: SET },
            AccessControl { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: ace::AUTHORITY_GET_ALL.into(), method_id: SET },
            AccessControl { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: ace::MBR_CONTROL_SET_DONE_TO_DOR.into(), method_id: SET },
            AccessControl { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: ace::DATA_STORE_GET_ALL.into(), method_id: SET },
            AccessControl { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: ace::DATA_STORE_SET_ALL.into(), method_id: SET },
            AccessControl { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION], ..Default::default() },
        ),
        // Authority
        (
            AccessControlRef { invoking_id: AUTHORITY.into(), method_id: NEXT },
            AccessControl { acl: vec![ace::ANYBODY], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: AUTHORITY.into(), method_id: GET },
            AccessControl { acl: vec![ace::AUTHORITY_GET_ALL, ace::ANYBODY_GET_COMMON_NAME], ..Default::default() },
        ),
        // C_PIN
        (
            AccessControlRef { invoking_id: C_PIN.into(), method_id: NEXT },
            AccessControl { acl: vec![ace::ANYBODY], ..Default::default() },
        ),
        // Locking
        (
            AccessControlRef { invoking_id: LOCKING.into(), method_id: NEXT },
            AccessControl { acl: vec![ace::ANYBODY], ..Default::default() },
        ),
        // MBRControl
        (
            AccessControlRef { invoking_id: MBR_CONTROL.into(), method_id: GET },
            AccessControl { acl: vec![ace::ANYBODY], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: MBR_CONTROL.into(), method_id: SET },
            AccessControl {
                acl: vec![
                    ace::MBR_CONTROL_ADMINS_SET,
                    ace::MBR_CONTROL_SET_DONE_TO_DOR,
                ],
                ..Default::default()
            },
        ),
        // MBR
        (
            AccessControlRef { invoking_id: MBR.into(), method_id: GET },
            AccessControl { acl: vec![ace::ANYBODY], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: MBR.into(), method_id: SET },
            AccessControl { acl: vec![ace::ADMIN], ..Default::default() },
        ),
    ];

    let admins = ADMINS.map(|admin_idx| {
        let admin_set_acl = if admin_idx == 1 {
            vec![ace::ADMINS_SET_COMMON_NAME]
        } else {
            vec![ace::ADMINS_SET_COMMON_NAME, ace::AUTHORITY_SET_ENABLED]
        };
        [
            // Authority
            (
                AccessControlRef { invoking_id: authority::ADMIN.get(admin_idx).unwrap().into(), method_id: SET },
                AccessControl { acl: admin_set_acl, ..Default::default() },
            ),
            // C_PIN
            (
                AccessControlRef { invoking_id: c_pin::ADMIN.get(admin_idx).unwrap().into(), method_id: GET },
                AccessControl { acl: vec![ace::C_PIN_ADMINS_GET_ALL_NOPIN], ..Default::default() },
            ),
            (
                AccessControlRef { invoking_id: c_pin::ADMIN.get(admin_idx).unwrap().into(), method_id: SET },
                AccessControl { acl: vec![ace::C_PIN_ADMINS_SET_PIN], ..Default::default() },
            ),
        ]
    });

    let users = USERS.map(|user_idx| {
        [
            // ACE
            (
                AccessControlRef { invoking_id: ace::C_PIN_USER_SET_PIN.get(user_idx).unwrap().into(), method_id: SET },
                AccessControl { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION], ..Default::default() },
            ),
            (
                AccessControlRef {
                    invoking_id: ace::USER_SET_COMMON_NAME.get(user_idx).unwrap().into(),
                    method_id: SET,
                },
                AccessControl { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION], ..Default::default() },
            ),
            // Authority
            (
                AccessControlRef { invoking_id: authority::USER.get(user_idx).unwrap().into(), method_id: SET },
                AccessControl {
                    acl: vec![
                        ace::AUTHORITY_SET_ENABLED,
                        ace::USER_SET_COMMON_NAME.get(user_idx).unwrap(),
                    ],
                    ..Default::default()
                },
            ),
            // C_PIN
            (
                AccessControlRef { invoking_id: c_pin::USER.get(user_idx).unwrap().into(), method_id: GET },
                AccessControl { acl: vec![ace::C_PIN_ADMINS_GET_ALL_NOPIN], ..Default::default() },
            ),
            (
                AccessControlRef { invoking_id: c_pin::USER.get(user_idx).unwrap().into(), method_id: SET },
                AccessControl { acl: vec![ace::C_PIN_USER_SET_PIN.get(user_idx).unwrap()], ..Default::default() },
            ),
        ]
    });

    let global_range = [
        // ACE
        (
            AccessControlRef { invoking_id: ace::K_AES_256_GLOBAL_RANGE_GEN_KEY.into(), method_id: SET },
            AccessControl { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION], ..Default::default() },
        ),
        (
            AccessControlRef {
                invoking_id: ace::LOCKING_GLOBAL_RANGE_GET_RANGE_START_TO_ACTIVE_KEY.into(),
                method_id: SET,
            },
            AccessControl { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: ace::LOCKING_GLOBAL_RANGE_SET_RD_LOCKED.into(), method_id: SET },
            AccessControl { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: ace::LOCKING_GLOBAL_RANGE_SET_WR_LOCKED.into(), method_id: SET },
            AccessControl { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION], ..Default::default() },
        ),
        // Locking
        (
            AccessControlRef { invoking_id: locking::GLOBAL_RANGE.into(), method_id: GET },
            AccessControl {
                acl: vec![
                    ace::LOCKING_GLOBAL_RANGE_GET_RANGE_START_TO_ACTIVE_KEY,
                    ace::ANYBODY_GET_COMMON_NAME,
                ],
                ..Default::default()
            },
        ),
        (
            AccessControlRef { invoking_id: locking::GLOBAL_RANGE.into(), method_id: SET },
            AccessControl {
                acl: vec![
                    ace::LOCKING_GLBL_RNG_ADMINS_SET,
                    ace::LOCKING_GLOBAL_RANGE_SET_RD_LOCKED,
                    ace::LOCKING_GLOBAL_RANGE_SET_WR_LOCKED,
                    ace::ADMINS_SET_COMMON_NAME,
                ],
                ..Default::default()
            },
        ),
        // K_AES_256
        (
            AccessControlRef { invoking_id: k_aes_256::GLOBAL_RANGE_KEY.into(), method_id: GEN_KEY },
            AccessControl { acl: vec![ace::K_AES_256_GLOBAL_RANGE_GEN_KEY], ..Default::default() },
        ),
    ];

    let ranges = RANGES.map(|range_idx| {
        [
            // ACE
            (
                AccessControlRef {
                    invoking_id: ace::K_AES_256_RANGE_GEN_KEY.get(range_idx).unwrap().into(),
                    method_id: SET,
                },
                AccessControl { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION], ..Default::default() },
            ),
            (
                AccessControlRef {
                    invoking_id: ace::LOCKING_RANGE_GET_RANGE_START_TO_ACTIVE_KEY.get(range_idx).unwrap().into(),
                    method_id: SET,
                },
                AccessControl { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION], ..Default::default() },
            ),
            (
                AccessControlRef {
                    invoking_id: ace::LOCKING_RANGE_SET_RD_LOCKED.get(range_idx).unwrap().into(),
                    method_id: SET,
                },
                AccessControl { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION], ..Default::default() },
            ),
            (
                AccessControlRef {
                    invoking_id: ace::LOCKING_RANGE_SET_WR_LOCKED.get(range_idx).unwrap().into(),
                    method_id: SET,
                },
                AccessControl { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION], ..Default::default() },
            ),
            // Locking
            (
                AccessControlRef { invoking_id: locking::RANGE.get(range_idx).unwrap().into(), method_id: GET },
                AccessControl {
                    acl: vec![
                        ace::LOCKING_RANGE_GET_RANGE_START_TO_ACTIVE_KEY.get(range_idx).unwrap(),
                        ace::ANYBODY_GET_COMMON_NAME,
                    ],
                    ..Default::default()
                },
            ),
            (
                AccessControlRef { invoking_id: locking::RANGE.get(range_idx).unwrap().into(), method_id: SET },
                AccessControl {
                    acl: vec![
                        ace::LOCKING_ADMINS_RANGE_START_TO_LOR,
                        ace::LOCKING_RANGE_SET_RD_LOCKED.get(range_idx).unwrap(),
                        ace::LOCKING_RANGE_SET_WR_LOCKED.get(range_idx).unwrap(),
                        ace::ADMINS_SET_COMMON_NAME,
                    ],
                    ..Default::default()
                },
            ),
            // K_AES_256
            (
                AccessControlRef {
                    invoking_id: k_aes_256::RANGE_KEY.get(range_idx).unwrap().into(),
                    method_id: GEN_KEY,
                },
                AccessControl { acl: vec![ace::K_AES_256_RANGE_GEN_KEY.get(range_idx).unwrap()], ..Default::default() },
            ),
        ]
    });

    fixed
        .into_iter()
        .chain(global_range)
        .chain(admins.into_iter().flatten())
        .chain(users.into_iter().flatten())
        .chain(ranges.into_iter().flatten())
        .collect()
}

pub fn ace() -> Table<Ace> {
    // Define column ranges used for multiple ACEs
    let range_start_to_active_key = LockingRange::RANGE_START..=LockingRange::ACTIVE_KEY;
    let range_start_to_lor = LockingRange::RANGE_START..=LockingRange::LOCK_ON_RESET;
    let range_admins_set = LockingRange::READ_LOCK_ENABLED..=LockingRange::LOCK_ON_RESET;

    let fixed = [
        // Base ACEs
        Ace {
            uid: Some(ace::ANYBODY),
            boolean_expr: Some(ace_expr!((authority::ANYBODY))),
            columns: Some((0..32).collect()),
            ..Default::default()
        },
        Ace {
            uid: Some(ace::ADMIN),
            boolean_expr: Some(ace_expr!((authority::ADMINS))),
            columns: Some((0..32).collect()),
            ..Default::default()
        },
        Ace {
            uid: Some(ace::ANYBODY_GET_COMMON_NAME),
            boolean_expr: Some(ace_expr!((authority::ANYBODY))),
            columns: Some([0, 2].into()), // UID, CommonName
            ..Default::default()
        },
        Ace {
            uid: Some(ace::ADMINS_SET_COMMON_NAME),
            boolean_expr: Some(ace_expr!((authority::ADMINS))),
            columns: Some([2].into()), // CommonName
            ..Default::default()
        },
        // ACE
        Ace {
            uid: Some(ace::ACE_GET_ALL),
            boolean_expr: Some(ace_expr!((authority::ADMINS))),
            columns: Some(Ace::all_columns().collect()),
            ..Default::default()
        },
        Ace {
            uid: Some(ace::ACE_SET_BOOLEAN_EXPRESSION),
            boolean_expr: Some(ace_expr!((authority::ADMINS))),
            columns: Some([Ace::BOOLEAN_EXPR].into()),
            ..Default::default()
        },
        // Authority
        Ace {
            uid: Some(ace::AUTHORITY_GET_ALL),
            boolean_expr: Some(ace_expr!((authority::ADMINS))),
            columns: Some(Authority::all_columns().collect()),
            ..Default::default()
        },
        Ace {
            uid: Some(ace::AUTHORITY_SET_ENABLED),
            boolean_expr: Some(ace_expr!((authority::ADMINS))),
            columns: Some([Authority::ENABLED].into()),
            ..Default::default()
        },
        // C_PIN
        Ace {
            uid: Some(ace::C_PIN_ADMINS_GET_ALL_NOPIN),
            boolean_expr: Some(ace_expr!((authority::ADMINS))),
            columns: Some(
                [
                    CPin::UID,
                    CPin::CHAR_SET,
                    CPin::TRY_LIMIT,
                    CPin::TRIES,
                    CPin::PERSISTENCE,
                ]
                .into(),
            ),
            ..Default::default()
        },
        Ace {
            uid: Some(ace::C_PIN_ADMINS_SET_PIN),
            boolean_expr: Some(ace_expr!((authority::ADMINS))),
            columns: Some([CPin::PIN].into()),
            ..Default::default()
        },
        // K_AES_*
        Ace {
            uid: Some(ace::K_AES_MODE),
            boolean_expr: Some(ace_expr!((authority::ANYBODY))),
            columns: Some([0].into()), // MODE column
            ..Default::default()
        },
        // Locking
        Ace {
            uid: Some(ace::LOCKING_ADMINS_RANGE_START_TO_LOR),
            boolean_expr: Some(ace_expr!((authority::ADMINS))),
            columns: Some(range_start_to_lor.clone().into_iter().collect()),
            ..Default::default()
        },
        // MBRControl
        Ace {
            uid: Some(ace::MBR_CONTROL_ADMINS_SET),
            boolean_expr: Some(ace_expr!((authority::ADMINS))),
            columns: Some(
                [
                    MbrControl::ENABLE,
                    MbrControl::DONE,
                    MbrControl::DONE_ON_RESET,
                ]
                .into(),
            ),
            ..Default::default()
        },
        Ace {
            uid: Some(ace::MBR_CONTROL_SET_DONE_TO_DOR),
            boolean_expr: Some(ace_expr!((authority::ADMINS))),
            columns: Some([MbrControl::DONE, MbrControl::DONE_ON_RESET].into()),
            ..Default::default()
        },
        // DataStore
        Ace {
            uid: Some(ace::DATA_STORE_GET_ALL),
            boolean_expr: Some(ace_expr!((authority::ADMINS))),
            columns: Some((0..1).collect()),
            ..Default::default()
        },
        Ace {
            uid: Some(ace::DATA_STORE_SET_ALL),
            boolean_expr: Some(ace_expr!((authority::ADMINS))),
            columns: Some((0..1).collect()),
            ..Default::default()
        },
    ];

    let users = USERS.map(|user_idx| {
        [
            // Authority
            Ace {
                uid: Some(ace::USER_SET_COMMON_NAME.get(user_idx).unwrap()),
                boolean_expr: Some(ace_expr!((authority::ADMINS))),
                columns: Some([Authority::COMMON_NAME].into()),
                ..Default::default()
            },
            // C_PIN
            Ace {
                uid: Some(ace::C_PIN_USER_SET_PIN.get(user_idx).unwrap()),
                boolean_expr: Some(ace_expr!((authority::ADMINS) (authority::USER.get(user_idx).unwrap()) ||)),
                columns: Some([CPin::PIN].into()),
                ..Default::default()
            },
        ]
    });

    let global_range = [
        // K_AES_256
        Ace {
            uid: Some(ace::K_AES_256_GLOBAL_RANGE_GEN_KEY),
            boolean_expr: Some(ace_expr!((authority::ADMINS))),
            columns: Some(KAes256::all_columns().collect()),
            ..Default::default()
        },
        // Locking
        Ace {
            uid: Some(ace::LOCKING_GLOBAL_RANGE_GET_RANGE_START_TO_ACTIVE_KEY),
            boolean_expr: Some(ace_expr!((authority::ADMINS))),
            columns: Some(range_start_to_active_key.clone().into_iter().collect()),
            ..Default::default()
        },
        Ace {
            uid: Some(ace::LOCKING_GLOBAL_RANGE_SET_RD_LOCKED),
            boolean_expr: Some(ace_expr!((authority::ADMINS))),
            columns: Some([LockingRange::READ_LOCKED].into()),
            ..Default::default()
        },
        Ace {
            uid: Some(ace::LOCKING_GLOBAL_RANGE_SET_WR_LOCKED),
            boolean_expr: Some(ace_expr!((authority::ADMINS))),
            columns: Some([LockingRange::WRITE_LOCKED].into()),
            ..Default::default()
        },
        Ace {
            uid: Some(ace::LOCKING_GLBL_RNG_ADMINS_SET),
            boolean_expr: Some(ace_expr!((authority::ADMINS))),
            columns: Some(range_admins_set.clone().into_iter().collect()),
            ..Default::default()
        },
    ];

    let ranges = RANGES.map(|range_idx| {
        [
            // K_AES_256
            Ace {
                uid: Some(ace::K_AES_256_RANGE_GEN_KEY.get(range_idx).unwrap()),
                boolean_expr: Some(ace_expr!((authority::ADMINS))),
                columns: Some(KAes256::all_columns().collect()),
                ..Default::default()
            },
            // Locking
            Ace {
                uid: Some(ace::LOCKING_RANGE_GET_RANGE_START_TO_ACTIVE_KEY.get(range_idx).unwrap()),
                boolean_expr: Some(ace_expr!((authority::ADMINS))),
                columns: Some(range_start_to_active_key.clone().into_iter().collect()),
                ..Default::default()
            },
            Ace {
                uid: Some(ace::LOCKING_RANGE_SET_RD_LOCKED.get(range_idx).unwrap()),
                boolean_expr: Some(ace_expr!((authority::ADMINS))),
                columns: Some([LockingRange::READ_LOCKED].into()),
                ..Default::default()
            },
            Ace {
                uid: Some(ace::LOCKING_RANGE_SET_WR_LOCKED.get(range_idx).unwrap()),
                boolean_expr: Some(ace_expr!((authority::ADMINS))),
                columns: Some([LockingRange::WRITE_LOCKED].into()),
                ..Default::default()
            },
            Ace {
                uid: Some(ace::LOCKING_ADMINS_RANGE_START_TO_LOR),
                boolean_expr: Some(ace_expr!((authority::ADMINS))),
                columns: Some(range_start_to_lor.clone().into_iter().collect()),
                ..Default::default()
            },
        ]
    });

    fixed
        .into_iter()
        .chain(global_range)
        .chain(users.into_iter().flatten())
        .chain(ranges.into_iter().flatten())
        .into_table()
        .expect("object missing an UID")
}

pub fn authority() -> Table<Authority> {
    let fixed = [
        Authority {
            uid: Some(authority::ANYBODY),
            name: Some("Anybody".into()),
            is_class: Some(false),
            class: Some(AuthorityRef::null()),
            enabled: Some(true),
            ..Default::default()
        },
        Authority {
            uid: Some(authority::ADMINS),
            name: Some("Admins".into()),
            is_class: Some(true),
            class: Some(AuthorityRef::null()),
            enabled: Some(true),
            ..Default::default()
        },
        Authority {
            uid: Some(authority::USERS),
            name: Some("Users".into()),
            is_class: Some(true),
            class: Some(AuthorityRef::null()),
            enabled: Some(true),
            ..Default::default()
        },
    ];

    let admins = ADMINS.map(|admin_idx| Authority {
        uid: Some(authority::ADMIN.get(admin_idx).unwrap()),
        name: Some(format!("Admin{}", admin_idx).into()),
        is_class: Some(false),
        class: Some(authority::ADMINS),
        enabled: Some(admin_idx == 0),
        operation: Some(AuthMethod::Password),
        credential: Some(c_pin::ADMIN.get(admin_idx).unwrap().into()),
        ..Default::default()
    });

    let users = USERS.map(|user_idx| Authority {
        uid: Some(authority::USER.get(user_idx).unwrap()),
        name: Some(format!("User{}", user_idx).into()),
        is_class: Some(false),
        class: Some(authority::USERS),
        enabled: Some(false),
        operation: Some(AuthMethod::Password),
        credential: Some(c_pin::USER.get(user_idx).unwrap().into()),
        ..Default::default()
    });

    fixed.into_iter().chain(admins).chain(users).into_table().expect("object missing an UID")
}

pub fn c_pin() -> Table<CPin> {
    let admins = ADMINS.map(|admin_idx| CPin {
        uid: Some(c_pin::ADMIN.get(admin_idx).unwrap()),
        pin: Some("default_admin_pw".as_bytes().into()),
        ..Default::default()
    });

    let users = USERS.map(|user_idx| CPin {
        uid: Some(c_pin::USER.get(user_idx).unwrap()),
        pin: Some("default_user_pw".as_bytes().into()),
        ..Default::default()
    });

    admins.into_iter().chain(users).into_table().expect("object missing an UID")
}

pub fn k_aes_256() -> Table<KAes256> {
    let fixed = [KAes256 { uid: Some(k_aes_256::GLOBAL_RANGE_KEY), ..Default::default() }];

    let ranges = RANGES
        .map(|range_idx| KAes256 { uid: Some(k_aes_256::RANGE_KEY.get(range_idx).unwrap()), ..Default::default() });

    fixed.into_iter().chain(ranges).into_table().expect("object missing an UID")
}

pub fn locking() -> Table<LockingRange> {
    let fixed = [LockingRange {
        uid: Some(locking::GLOBAL_RANGE),
        active_key: Some(k_aes_256::GLOBAL_RANGE_KEY),
        ..Default::default()
    }];

    let ranges = RANGES.map(|range_idx| LockingRange {
        uid: Some(locking::RANGE.get(range_idx).unwrap()),
        active_key: Some(k_aes_256::RANGE_KEY.get(range_idx).unwrap()),
        ..Default::default()
    });

    fixed.into_iter().chain(ranges).into_table().expect("object missing an UID")
}

pub fn mbr_control() -> Table<MbrControl> {
    [MbrControl { uid: Some(mbr_control::MBR_CONTROL), ..Default::default() }]
        .into_table()
        .expect("object missing an UID")
}

pub fn table() -> Table<TableDesc> {
    [
        TableDesc {
            uid: Some(table::MBR_CONTROL),
            name: Some("MBRControl".into()),
            kind: Some(TableKind::Object),
            ..Default::default()
        },
        TableDesc {
            uid: Some(table::MBR),
            name: Some("MBR".into()),
            kind: Some(TableKind::Byte),
            rows: Some(MBR_SIZE),
            ..Default::default()
        },
    ]
    .into_table()
    .expect("object missing an UID")
}
