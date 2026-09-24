//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::ops::Range;

use sed_spec::{
    ace_expr,
    objects::{AccessControl, AccessControlRef, Ace, Authority, AuthorityRef, CPin, KAes256, LockingRange, TableDesc},
    preconfig::{
        core::shared::invoking_id::THIS_SP,
        enterprise::{
            admin::sp,
            locking::{ace, authority, c_pin, k_aes_256, locking},
        },
    },
    types::AuthMethod,
};

use crate::tper::{
    Locking,
    preconfig_shared::{INITIAL_SID_PASSWORD, IntoTable},
    security_provider::Table,
};

// Only BandMaster0 (Global_Range) and BandMaster1..=8 (Band1..Band8) are
// preconfigured, out of the 1024/2048 UIDs reserved by the spec. This mirrors
// how the Opal 2 preconfig only sets up 8 non-global ranges.
const BANDS: Range<usize> = 1..9;
const DATA_STORE_SIZE: usize = 0x00100000;

pub fn preconfig() -> Locking {
    Locking {
        uid: sp::LOCKING,
        access_control: access_control(),
        ace: ace(),
        authority: authority(),
        c_pin: c_pin(),
        // Neither MBRControl nor MBR are defined by this SSC: the tables are
        // left empty so that Get/Set on them fails with InvalidParameter,
        // same as on a real device that doesn't implement shadow MBR.
        k_aes_256: k_aes_256(),
        locking: locking(),
        mbr_control: Table::new(),
        table: table(),
        mbr: Vec::new(),
        data_store: vec![vec![0u8; DATA_STORE_SIZE]],
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
            AccessControl { acl: vec![ace::ANYBODY], ..Default::default() },
        ),
        // Authority
        (
            AccessControlRef { invoking_id: AUTHORITY.into(), method_id: NEXT },
            AccessControl { acl: vec![ace::ANY_MASTER], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: authority::ANYBODY.into(), method_id: GET },
            AccessControl { acl: vec![ace::ANYBODY], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: authority::BAND_MASTERS.into(), method_id: GET },
            AccessControl { acl: vec![ace::ANY_MASTER], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: authority::ERASE_MASTER.into(), method_id: GET },
            AccessControl { acl: vec![ace::ERASE_MASTER], ..Default::default() },
        ),
        // C_PIN
        (
            AccessControlRef { invoking_id: C_PIN.into(), method_id: NEXT },
            AccessControl { acl: vec![ace::ANY_MASTER], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: c_pin::ERASE_MASTER.into(), method_id: SET },
            AccessControl { acl: vec![ace::ERASE_MASTER_SET_SELF], ..Default::default() },
        ),
        // Locking
        (
            AccessControlRef { invoking_id: LOCKING.into(), method_id: NEXT },
            AccessControl { acl: vec![ace::ANY_MASTER], ..Default::default() },
        ),
        // DataStore
        (
            AccessControlRef { invoking_id: DATA_STORE.get(0).unwrap().into(), method_id: GET },
            AccessControl { acl: vec![ace::ANYBODY], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: DATA_STORE.get(0).unwrap().into(), method_id: SET },
            AccessControl { acl: vec![ace::BAND_MASTERS], ..Default::default() },
        ),
    ];

    let global_band = [
        // Authority
        (
            AccessControlRef { invoking_id: authority::BAND_MASTER.get(0).unwrap().into(), method_id: GET },
            AccessControl { acl: vec![ace::BAND_MASTER.get(0).unwrap()], ..Default::default() },
        ),
        // C_PIN
        (
            AccessControlRef { invoking_id: c_pin::BAND_MASTER.get(0).unwrap().into(), method_id: SET },
            AccessControl { acl: vec![ace::BAND_MASTER_SET_SELF.get(0).unwrap()], ..Default::default() },
        ),
        // Locking
        (
            AccessControlRef { invoking_id: locking::GLOBAL_RANGE.into(), method_id: GET },
            AccessControl { acl: vec![ace::ANYBODY_GET_BAND], ..Default::default() },
        ),
        (
            AccessControlRef { invoking_id: locking::GLOBAL_RANGE.into(), method_id: SET },
            AccessControl { acl: vec![ace::BAND_MASTER_SET_BAND.get(0).unwrap()], ..Default::default() },
        ),
        // K_AES_256
        (
            AccessControlRef { invoking_id: k_aes_256::GLOBAL_RANGE_KEY.into(), method_id: GET },
            AccessControl { acl: vec![ace::GET_K_AES_MODE], ..Default::default() },
        ),
    ];

    let bands = BANDS.map(|band_idx| {
        [
            // Authority
            (
                AccessControlRef { invoking_id: authority::BAND_MASTER.get(band_idx).unwrap().into(), method_id: GET },
                AccessControl { acl: vec![ace::BAND_MASTER.get(band_idx).unwrap()], ..Default::default() },
            ),
            // C_PIN
            (
                AccessControlRef { invoking_id: c_pin::BAND_MASTER.get(band_idx).unwrap().into(), method_id: SET },
                AccessControl { acl: vec![ace::BAND_MASTER_SET_SELF.get(band_idx).unwrap()], ..Default::default() },
            ),
            // Locking
            (
                AccessControlRef { invoking_id: locking::BAND.get(band_idx - 1).unwrap().into(), method_id: GET },
                AccessControl { acl: vec![ace::ANYBODY_GET_BAND], ..Default::default() },
            ),
            (
                AccessControlRef { invoking_id: locking::BAND.get(band_idx - 1).unwrap().into(), method_id: SET },
                AccessControl { acl: vec![ace::BAND_MASTER_SET_BAND.get(band_idx).unwrap()], ..Default::default() },
            ),
            // K_AES_256
            (
                AccessControlRef { invoking_id: k_aes_256::BAND_KEY.get(band_idx - 1).unwrap().into(), method_id: GET },
                AccessControl { acl: vec![ace::GET_K_AES_MODE], ..Default::default() },
            ),
        ]
    });

    fixed.into_iter().chain(global_band).chain(bands.into_iter().flatten()).collect()
}

pub fn ace() -> Table<Ace> {
    // BandMaster0's Set ACL cannot resize the Global Range, only toggle its
    // lock state. BandMaster{n>=1} can also move/resize their Band.
    let global_band_set = LockingRange::READ_LOCK_ENABLED..=LockingRange::LOCK_ON_RESET;
    let band_set = LockingRange::RANGE_START..=LockingRange::LOCK_ON_RESET;
    let get_band = LockingRange::UID..=LockingRange::ACTIVE_KEY;

    let fixed = [
        Ace {
            uid: Some(ace::ANYBODY),
            boolean_expr: Some(ace_expr!((authority::ANYBODY))),
            columns: Some((0..32).collect()),
            ..Default::default()
        },
        Ace {
            uid: Some(ace::ANY_MASTER),
            boolean_expr: Some(ace_expr!((authority::BAND_MASTERS) (authority::ERASE_MASTER) ||)),
            columns: Some((0..32).collect()),
            ..Default::default()
        },
        Ace {
            uid: Some(ace::BAND_MASTERS),
            boolean_expr: Some(ace_expr!((authority::BAND_MASTERS))),
            columns: Some((0..32).collect()),
            ..Default::default()
        },
        Ace {
            uid: Some(ace::ERASE_MASTER),
            boolean_expr: Some(ace_expr!((authority::ERASE_MASTER))),
            columns: Some((0..32).collect()),
            ..Default::default()
        },
        Ace {
            uid: Some(ace::ERASE_MASTER_SET_SELF),
            boolean_expr: Some(ace_expr!((authority::ERASE_MASTER))),
            columns: Some([CPin::PIN].into()),
            ..Default::default()
        },
        // Locking
        Ace {
            uid: Some(ace::ANYBODY_GET_BAND),
            boolean_expr: Some(ace_expr!((authority::ANYBODY))),
            columns: Some(get_band.clone().into_iter().collect()),
            ..Default::default()
        },
        // K_AES_256
        Ace {
            uid: Some(ace::GET_K_AES_MODE),
            boolean_expr: Some(ace_expr!((authority::ANYBODY))),
            columns: Some([KAes256::MODE].into()),
            ..Default::default()
        },
    ];

    let global_band = [
        Ace {
            uid: Some(ace::BAND_MASTER.get(0).unwrap()),
            boolean_expr: Some(ace_expr!((authority::BAND_MASTER.get(0).unwrap()))),
            columns: Some((0..32).collect()),
            ..Default::default()
        },
        Ace {
            uid: Some(ace::BAND_MASTER_SET_SELF.get(0).unwrap()),
            boolean_expr: Some(ace_expr!((authority::BAND_MASTER.get(0).unwrap()))),
            columns: Some([CPin::PIN].into()),
            ..Default::default()
        },
        Ace {
            uid: Some(ace::BAND_MASTER_SET_BAND.get(0).unwrap()),
            boolean_expr: Some(ace_expr!((authority::BAND_MASTER.get(0).unwrap()))),
            columns: Some(global_band_set.clone().into_iter().collect()),
            ..Default::default()
        },
    ];

    let bands = BANDS.map(|band_idx| {
        [
            Ace {
                uid: Some(ace::BAND_MASTER.get(band_idx).unwrap()),
                boolean_expr: Some(ace_expr!((authority::BAND_MASTER.get(band_idx).unwrap()))),
                columns: Some((0..32).collect()),
                ..Default::default()
            },
            Ace {
                uid: Some(ace::BAND_MASTER_SET_SELF.get(band_idx).unwrap()),
                boolean_expr: Some(ace_expr!((authority::BAND_MASTER.get(band_idx).unwrap()))),
                columns: Some([CPin::PIN].into()),
                ..Default::default()
            },
            Ace {
                uid: Some(ace::BAND_MASTER_SET_BAND.get(band_idx).unwrap()),
                boolean_expr: Some(ace_expr!((authority::BAND_MASTER.get(band_idx).unwrap()))),
                columns: Some(band_set.clone().into_iter().collect()),
                ..Default::default()
            },
        ]
    });

    fixed
        .into_iter()
        .chain(global_band)
        .chain(bands.into_iter().flatten())
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
            uid: Some(authority::BAND_MASTERS),
            name: Some("BandMasters".into()),
            is_class: Some(true),
            class: Some(AuthorityRef::null()),
            enabled: Some(true),
            ..Default::default()
        },
        Authority {
            uid: Some(authority::ERASE_MASTER),
            name: Some("EraseMaster".into()),
            is_class: Some(false),
            class: Some(AuthorityRef::null()),
            enabled: Some(true),
            operation: Some(AuthMethod::Password),
            credential: Some(c_pin::ERASE_MASTER.into()),
            ..Default::default()
        },
    ];

    let band_masters = (0..9).map(|band_master_idx| Authority {
        uid: Some(authority::BAND_MASTER.get(band_master_idx).unwrap()),
        name: Some(format!("BandMaster{}", band_master_idx).into()),
        is_class: Some(false),
        class: Some(authority::BAND_MASTERS),
        enabled: Some(true),
        operation: Some(AuthMethod::Password),
        credential: Some(c_pin::BAND_MASTER.get(band_master_idx).unwrap().into()),
        ..Default::default()
    });

    fixed.into_iter().chain(band_masters).into_table().expect("object missing an UID")
}

pub fn c_pin() -> Table<CPin> {
    // Per the spec, the PIN value for all ranges SHALL be set to the MSID
    // Credential value at manufacturing time.
    let erase_master = CPin { uid: Some(c_pin::ERASE_MASTER), pin: Some(INITIAL_SID_PASSWORD), ..Default::default() };

    let band_masters = (0..9).map(|band_master_idx| CPin {
        uid: Some(c_pin::BAND_MASTER.get(band_master_idx).unwrap()),
        pin: Some(INITIAL_SID_PASSWORD),
        ..Default::default()
    });

    [erase_master].into_iter().chain(band_masters).into_table().expect("object missing an UID")
}

pub fn k_aes_256() -> Table<KAes256> {
    let fixed = [KAes256 { uid: Some(k_aes_256::GLOBAL_RANGE_KEY), ..Default::default() }];

    let bands = BANDS
        .map(|band_idx| KAes256 { uid: Some(k_aes_256::BAND_KEY.get(band_idx - 1).unwrap()), ..Default::default() });

    fixed.into_iter().chain(bands).into_table().expect("object missing an UID")
}

pub fn locking() -> Table<LockingRange> {
    let fixed = [LockingRange {
        uid: Some(locking::GLOBAL_RANGE),
        active_key: Some(k_aes_256::GLOBAL_RANGE_KEY),
        ..Default::default()
    }];

    let bands = BANDS.map(|band_idx| LockingRange {
        uid: Some(locking::BAND.get(band_idx - 1).unwrap()),
        active_key: Some(k_aes_256::BAND_KEY.get(band_idx - 1).unwrap()),
        ..Default::default()
    });

    fixed.into_iter().chain(bands).into_table().expect("object missing an UID")
}

pub fn table() -> Table<TableDesc> {
    [].into_table().expect("object missing an UID")
}
