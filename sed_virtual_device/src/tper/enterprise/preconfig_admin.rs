//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_spec::{
    ace_expr,
    objects::{AccessControl, AccessControlRef, Ace, Authority, AuthorityRef, CPin, SecurityProvider, TableDesc},
    preconfig::{
        core::shared::invoking_id::THIS_SP,
        enterprise::admin::{ace, authority, c_pin, sp},
        psid,
    },
    types::{AuthMethod, LifeCycleState},
};

use crate::tper::{
    Admin,
    preconfig_shared::{INITIAL_SID_PASSWORD, IntoTable, PSID_PASSWORD},
    security_provider::Table,
};

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

    [
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
            AccessControl { acl: vec![ace::MAKERS], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: authority::ANYBODY.into(), method_id: GET },
            AccessControl { acl: vec![ace::ANYBODY], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: authority::MAKERS.into(), method_id: GET },
            AccessControl { acl: vec![ace::MAKERS], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: authority::MAKERS.into(), method_id: SET },
            AccessControl { acl: vec![ace::SID_SET_MAKERS], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: authority::SID.into(), method_id: GET },
            AccessControl { acl: vec![ace::SID], ..Default::default() },
        ),
        // C_PIN
        (
            AccessControlRef { invoking_id: C_PIN.into(), method_id: NEXT },
            AccessControl { acl: vec![ace::MAKERS], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: c_pin::SID.into(), method_id: SET },
            AccessControl { acl: vec![ace::SID_SET_SELF], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: c_pin::MSID.into(), method_id: GET },
            AccessControl { acl: vec![ace::MSID_GET], ..Default::default() },
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
    ]
    .into_iter()
    .collect()
}

// Unlike other SSCs, this SSC's Admin SP doesn't define a Revert method (see
// `docs/specification/SSC_Enterprise_v1.01.md`, Table 27), so no PSID/SID
// all-columns ACE is needed to gate it.
pub fn ace() -> Table<Ace> {
    [
        // Base ACEs
        Ace {
            uid: Some(ace::ANYBODY),
            boolean_expr: Some(ace_expr!((authority::ANYBODY))),
            columns: Some((0..32).collect()),
            ..Default::default()
        },
        Ace {
            uid: Some(ace::MAKERS),
            boolean_expr: Some(ace_expr!((authority::MAKERS))),
            columns: Some((0..32).collect()),
            ..Default::default()
        },
        Ace {
            uid: Some(ace::SID),
            boolean_expr: Some(ace_expr!((authority::SID))),
            columns: Some((0..32).collect()),
            ..Default::default()
        },
        // Authority
        Ace {
            uid: Some(ace::SID_SET_MAKERS),
            boolean_expr: Some(ace_expr!((authority::SID))),
            columns: Some([Authority::ENABLED].into()),
            ..Default::default()
        },
        // C_PIN
        Ace {
            uid: Some(ace::SID_SET_SELF),
            boolean_expr: Some(ace_expr!((authority::SID))),
            columns: Some([CPin::PIN].into()),
            ..Default::default()
        },
        Ace {
            uid: Some(ace::MSID_GET),
            boolean_expr: Some(ace_expr!((authority::ANYBODY))),
            columns: Some([CPin::PIN].into()),
            ..Default::default()
        },
    ]
    .into_table()
    .expect("object missing an UID")
}

pub fn authority() -> Table<Authority> {
    [
        Authority {
            uid: Some(authority::ANYBODY),
            name: Some("Anybody".into()),
            is_class: Some(false),
            class: Some(AuthorityRef::null()),
            ..Default::default()
        },
        Authority {
            uid: Some(authority::MAKERS),
            name: Some("Makers".into()),
            is_class: Some(true),
            class: Some(AuthorityRef::null()),
            ..Default::default()
        },
        // MakerSymK authenticates via a symmetric-key credential, which isn't
        // modeled by the virtual device. It's included for structural
        // completeness, but cannot actually be authenticated as.
        Authority {
            uid: Some(authority::MAKER_SYM_K),
            name: Some("MakerSymK".into()),
            is_class: Some(false),
            class: Some(authority::MAKERS),
            ..Default::default()
        },
        Authority {
            uid: Some(authority::SID),
            name: Some("SID".into()),
            is_class: Some(false),
            class: Some(AuthorityRef::null()),
            operation: Some(AuthMethod::Password),
            credential: Some(c_pin::SID.into()),
            ..Default::default()
        },
        Authority {
            uid: Some(psid::admin::authority::PSID),
            name: Some("PSID".into()),
            is_class: Some(false),
            class: Some(AuthorityRef::null()),
            operation: Some(AuthMethod::Password),
            credential: Some(psid::admin::c_pin::PSID.into()),
            ..Default::default()
        },
    ]
    .into_table()
    .expect("object missing an UID")
}

pub fn c_pin() -> Table<CPin> {
    [
        CPin { uid: Some(c_pin::SID), pin: Some(INITIAL_SID_PASSWORD), ..Default::default() },
        CPin { uid: Some(c_pin::MSID), pin: Some(INITIAL_SID_PASSWORD), ..Default::default() },
        CPin { uid: Some(psid::admin::c_pin::PSID), pin: Some(PSID_PASSWORD), ..Default::default() },
    ]
    .into_table()
    .expect("object missing an UID")
}

pub fn sp() -> Table<SecurityProvider> {
    [
        SecurityProvider {
            uid: Some(sp::ADMIN),
            name: Some("Admin".into()),
            life_cycle_state: Some(LifeCycleState::Manufactured),
            ..Default::default()
        },
        // The Locking SP has no Activate method in this SSC: it's usable as
        // soon as the TPer is manufactured, without a separate activation step.
        SecurityProvider {
            uid: Some(sp::LOCKING),
            name: Some("Locking".into()),
            life_cycle_state: Some(LifeCycleState::Manufactured),
            ..Default::default()
        },
    ]
    .into_table()
    .expect("object missing an UID")
}

pub fn table() -> Table<TableDesc> {
    [].into_table().expect("object missing an UID")
}
