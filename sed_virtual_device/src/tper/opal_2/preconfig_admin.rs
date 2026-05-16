use std::ops::Range;

use sed_spec::{
    ace_expr,
    objects::{AccessControl, AccessControlRef, Ace, Authority, CPin, SecurityProvider, TableDesc},
    preconfig::{
        core::shared::invoking_id::THIS_SP,
        opal_2::admin::{ace, authority, c_pin, sp},
        psid,
    },
    types::{AuthMethod, LifeCycleState},
};

use crate::tper::{
    Admin,
    preconfig_shared::{AllColumns, INITIAL_SID_PASSWORD, IntoTable, PSID_PASSWORD},
    security_provider::Table,
};

const ADMINS: Range<usize> = 0..4;

pub fn preconfig() -> Admin {
    Admin {
        uid: sp::ADMIN,
        access_control: access_control(),
        ace: ace(),
        authority: authority(),
        c_pin: c_pin(),
        sp: sp(),
        table: table(),
    }
}

pub fn access_control() -> Table<AccessControl> {
    use sed_spec::preconfig::core::shared::{method_id::*, table_id::*};

    let fixed = [
        // Table
        (
            AccessControlRef { invoking_id: TABLE.into(), method_id: NEXT },
            AccessControl { acl: vec![ace::ANYBODY], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: TABLE.into(), method_id: GET },
            AccessControl { acl: vec![ace::ANYBODY], ..Default::default() },
        ),
        // AccessControl
        (
            AccessControlRef { invoking_id: ACCESS_CONTROL.into(), method_id: GET_ACL },
            AccessControl { acl: vec![ace::ANYBODY], ..Default::default() },
        ),
        // ACE
        (
            AccessControlRef { invoking_id: ACE.into(), method_id: NEXT },
            AccessControl { acl: vec![ace::ANYBODY], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: ACE.into(), method_id: GET },
            AccessControl { acl: vec![ace::ANYBODY], ..Default::default() },
        ),
        // Authority
        (
            AccessControlRef { invoking_id: AUTHORITY.into(), method_id: NEXT },
            AccessControl { acl: vec![ace::ANYBODY], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: AUTHORITY.into(), method_id: GET },
            AccessControl { acl: vec![ace::ANYBODY], ..Default::default() },
        ),
        // C_PIN
        (
            AccessControlRef { invoking_id: C_PIN.into(), method_id: NEXT },
            AccessControl { acl: vec![ace::ANYBODY], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: c_pin::SID.into(), method_id: GET },
            AccessControl { acl: vec![ace::C_PIN_SID_GET_NOPIN], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: c_pin::SID.into(), method_id: SET },
            AccessControl { acl: vec![ace::C_PIN_SID_SET_PIN], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: c_pin::MSID.into(), method_id: GET },
            AccessControl { acl: vec![ace::C_PIN_MSID_GET_PIN], ..Default::default() },
        ),
        // SP
        (
            AccessControlRef { invoking_id: THIS_SP.into(), method_id: AUTHENTICATE },
            AccessControl { acl: vec![ace::ANYBODY], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: THIS_SP.into(), method_id: RANDOM },
            AccessControl { acl: vec![ace::ANYBODY], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: SP.into(), method_id: NEXT },
            AccessControl { acl: vec![ace::ANYBODY], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: SP.into(), method_id: GET },
            AccessControl { acl: vec![ace::ANYBODY], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: SP.into(), method_id: REVERT },
            AccessControl { acl: vec![ace::SP_SID, ace::ADMIN, psid::admin::ace::SP_PSID], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: SP.into(), method_id: ACTIVATE },
            AccessControl { acl: vec![ace::SP_SID], ..Default::default() },
        ),
    ];

    let admins = ADMINS.map(|admin_idx| {
        // Authority
        [
            (
                AccessControlRef { invoking_id: authority::ADMIN.get(admin_idx).unwrap().into(), method_id: SET },
                AccessControl { acl: vec![ace::SET_ENABLED].into(), ..Default::default() },
            ),
            (
                AccessControlRef { invoking_id: c_pin::ADMIN.get(admin_idx).unwrap().into(), method_id: GET },
                AccessControl { acl: vec![ace::C_PIN_SID_GET_NOPIN].into(), ..Default::default() },
            ),
            (
                AccessControlRef { invoking_id: c_pin::ADMIN.get(admin_idx).unwrap().into(), method_id: SET },
                AccessControl { acl: vec![ace::C_PIN_ADMINS_SET_PIN].into(), ..Default::default() },
            ),
        ]
    });

    fixed.into_iter().chain(admins.into_iter().flatten()).collect()
}

pub fn ace() -> Table<Ace> {
    [
        // Base ACEs
        Ace {
            uid: Some(ace::ANYBODY),
            boolean_expr: Some(ace_expr!((authority::ANYBODY))),
            columns: Some(Ace::all_columns().collect()),
            ..Default::default()
        },
        Ace {
            uid: Some(ace::ADMIN),
            boolean_expr: Some(ace_expr!((authority::ADMINS))),
            columns: Some(Ace::all_columns().collect()),
            ..Default::default()
        },
        // Authority table
        Ace {
            uid: Some(ace::SET_ENABLED),
            boolean_expr: Some(ace_expr!((authority::SID))),
            columns: Some([Authority::ENABLED].into()),
            ..Default::default()
        },
        // C_PIN table
        Ace {
            uid: Some(ace::C_PIN_SID_GET_NOPIN),
            boolean_expr: Some(ace_expr!((authority::ADMINS) (authority::SID) ||)),
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
            uid: Some(ace::C_PIN_SID_SET_PIN),
            boolean_expr: Some(ace_expr!((authority::SID))),
            columns: Some([CPin::PIN].into()),
            ..Default::default()
        },
        Ace {
            uid: Some(ace::C_PIN_MSID_GET_PIN),
            boolean_expr: Some(ace_expr!((authority::ANYBODY))),
            columns: Some([CPin::UID, CPin::PIN].into()),
            ..Default::default()
        },
        Ace {
            uid: Some(ace::C_PIN_ADMINS_SET_PIN),
            boolean_expr: Some(ace_expr!((authority::ADMINS) (authority::SID) ||)),
            columns: Some([CPin::PIN].into()),
            ..Default::default()
        },
        // SP
        Ace {
            uid: Some(ace::SP_SID),
            boolean_expr: Some(ace_expr!((authority::SID))),
            columns: Some(SecurityProvider::all_columns().collect()),
            ..Default::default()
        },
        Ace {
            uid: Some(psid::admin::ace::SP_PSID),
            boolean_expr: Some(ace_expr!((psid::admin::authority::PSID))),
            columns: Some(SecurityProvider::all_columns().collect()),
            ..Default::default()
        },
    ]
    .into_table()
    .expect("object missing an UID")
}

pub fn authority() -> Table<Authority> {
    let fixed = [
        Authority {
            uid: Some(authority::ANYBODY),
            name: Some("Anybody".into()),
            is_class: Some(false),
            ..Default::default()
        },
        Authority {
            uid: Some(authority::ADMINS),
            name: Some("Admins".into()),
            is_class: Some(true),
            ..Default::default()
        },
        Authority {
            uid: Some(authority::MAKERS),
            name: Some("Makers".into()),
            is_class: Some(true),
            ..Default::default()
        },
        Authority {
            uid: Some(authority::SID),
            name: Some("SID".into()),
            is_class: Some(false),
            operation: Some(AuthMethod::Password),
            credential: Some(c_pin::SID.into()),
            ..Default::default()
        },
        Authority {
            uid: Some(psid::admin::authority::PSID),
            name: Some("PSID".into()),
            is_class: Some(false),
            operation: Some(AuthMethod::Password.into()),
            credential: Some(psid::admin::c_pin::PSID.into()),
            ..Default::default()
        },
    ];

    let admins = ADMINS.map(|admin_idx| Authority {
        uid: Some(authority::ADMIN.get(admin_idx).unwrap()),
        name: Some(format!("Admin{}", admin_idx).into()),
        enabled: Some(false),
        is_class: Some(false),
        class: Some(authority::ADMINS),
        operation: Some(AuthMethod::Password),
        credential: Some(c_pin::ADMIN.get(admin_idx).unwrap().into()),
        ..Default::default()
    });

    fixed.into_iter().chain(admins).into_table().expect("object missing an UID")
}

pub fn c_pin() -> Table<CPin> {
    let fixed = [
        CPin { uid: Some(c_pin::SID), pin: Some(INITIAL_SID_PASSWORD), ..Default::default() },
        CPin { uid: Some(c_pin::MSID), pin: Some(INITIAL_SID_PASSWORD), ..Default::default() },
        CPin { uid: Some(psid::admin::c_pin::PSID), pin: Some(PSID_PASSWORD), ..Default::default() },
    ];

    let admins = ADMINS.map(|admin_idx| CPin {
        uid: Some(c_pin::ADMIN.get(admin_idx).unwrap()),
        pin: Some(b"random_password".as_slice().into()),
        ..Default::default()
    });

    fixed.into_iter().chain(admins).into_table().expect("object missing an UID")
}

pub fn sp() -> Table<SecurityProvider> {
    let fixed = [
        SecurityProvider {
            uid: Some(sp::ADMIN),
            name: Some("Admin".into()),
            life_cycle_state: Some(LifeCycleState::Manufactured),
            ..Default::default()
        },
        SecurityProvider {
            uid: Some(sp::LOCKING),
            name: Some("Locking".into()),
            life_cycle_state: Some(LifeCycleState::ManufacturedInactive),
            ..Default::default()
        },
    ];

    fixed.into_table().expect("object missing an UID")
}

pub fn table() -> Table<TableDesc> {
    [].into_table().expect("object missing an UID")
}
