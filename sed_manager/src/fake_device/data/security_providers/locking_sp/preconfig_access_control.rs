use crate::fake_device::data::access_control_table::{AccessControlEntry, AccessControlRef, AccessControlTable};
use crate::fake_device::data::security_providers::locking_sp::{ADMIN_IDX, RANGE_IDX, USER_IDX};
use crate::spec::opal::locking::*;

pub fn preconfig_access_control() -> AccessControlTable {
    use crate::spec::{method_id, table_id};
    let mut items = vec![
        // Table
        (
            AccessControlRef::new(table_id::TABLE.into(), method_id::NEXT),
            AccessControlEntry { acl: vec![ace::ANYBODY].into(), ..Default::default() },
        ),
        (
            AccessControlRef::new(table_id::TABLE.into(), method_id::GET),
            AccessControlEntry { acl: vec![ace::ANYBODY].into(), ..Default::default() },
        ),
        // ACE
        (
            AccessControlRef::new(table_id::ACE.into(), method_id::NEXT),
            AccessControlEntry { acl: vec![ace::ANYBODY].into(), ..Default::default() },
        ),
        (
            AccessControlRef::new(table_id::ACE.into(), method_id::GET),
            AccessControlEntry { acl: vec![ace::ACE_GET_ALL].into(), ..Default::default() },
        ),
        (
            AccessControlRef::new(ace::ACE_GET_ALL.into(), method_id::SET),
            AccessControlEntry { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION].into(), ..Default::default() },
        ),
        (
            AccessControlRef::new(ace::AUTHORITY_GET_ALL.into(), method_id::SET),
            AccessControlEntry { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION].into(), ..Default::default() },
        ),
        (
            AccessControlRef::new(ace::MBR_CONTROL_SET_DONE_TO_DOR.into(), method_id::SET),
            AccessControlEntry { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION].into(), ..Default::default() },
        ),
        (
            AccessControlRef::new(ace::DATA_STORE_GET_ALL.into(), method_id::SET),
            AccessControlEntry { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION].into(), ..Default::default() },
        ),
        (
            AccessControlRef::new(ace::DATA_STORE_SET_ALL.into(), method_id::SET),
            AccessControlEntry { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION].into(), ..Default::default() },
        ),
        // Authority
        (
            AccessControlRef::new(table_id::AUTHORITY.into(), method_id::NEXT),
            AccessControlEntry { acl: vec![ace::ANYBODY].into(), ..Default::default() },
        ),
        (
            AccessControlRef::new(table_id::AUTHORITY.into(), method_id::GET),
            AccessControlEntry {
                acl: vec![ace::AUTHORITY_GET_ALL, ace::ANYBODY_GET_COMMON_NAME].into(),
                ..Default::default()
            },
        ),
        // C_PIN
        (
            AccessControlRef::new(table_id::C_PIN.into(), method_id::NEXT),
            AccessControlEntry { acl: vec![ace::ANYBODY].into(), ..Default::default() },
        ),
        // Locking
        (
            AccessControlRef::new(table_id::LOCKING.into(), method_id::NEXT),
            AccessControlEntry { acl: vec![ace::ANYBODY].into(), ..Default::default() },
        ),
        // MBRControl
        (
            AccessControlRef::new(table_id::MBR_CONTROL.into(), method_id::GET),
            AccessControlEntry { acl: vec![ace::ANYBODY].into(), ..Default::default() },
        ),
        (
            AccessControlRef::new(table_id::MBR_CONTROL.into(), method_id::SET),
            AccessControlEntry {
                acl: vec![
                    ace::MBR_CONTROL_ADMINS_SET,
                    ace::MBR_CONTROL_SET_DONE_TO_DOR,
                ]
                .into(),
                ..Default::default()
            },
        ),
        // MBR
        (
            AccessControlRef::new(table_id::MBR.into(), method_id::GET),
            AccessControlEntry { acl: vec![ace::ANYBODY].into(), ..Default::default() },
        ),
        (
            AccessControlRef::new(table_id::MBR.into(), method_id::SET),
            AccessControlEntry { acl: vec![ace::ADMIN].into(), ..Default::default() },
        ),
    ];

    // Admins
    for admin_idx in ADMIN_IDX {
        // Authority
        items.push((
            AccessControlRef::new(authority::ADMIN.nth(admin_idx).unwrap().into(), method_id::SET),
            AccessControlEntry { acl: vec![ace::ADMINS_SET_COMMON_NAME].into(), ..Default::default() },
        ));
        // C_PIN
        items.push((
            AccessControlRef::new(c_pin::ADMIN.nth(admin_idx).unwrap().into(), method_id::GET),
            AccessControlEntry { acl: vec![ace::C_PIN_ADMINS_GET_ALL_NOPIN].into(), ..Default::default() },
        ));
        items.push((
            AccessControlRef::new(c_pin::ADMIN.nth(admin_idx).unwrap().into(), method_id::SET),
            AccessControlEntry { acl: vec![ace::C_PIN_ADMINS_SET_PIN].into(), ..Default::default() },
        ));
    }

    // Users
    for user_idx in USER_IDX {
        // ACE
        items.push((
            AccessControlRef::new(ace::C_PIN_USER_SET_PIN.nth(user_idx).unwrap().into(), method_id::SET),
            AccessControlEntry { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION].into(), ..Default::default() },
        ));
        items.push((
            AccessControlRef::new(ace::USER_SET_COMMON_NAME.nth(user_idx).unwrap().into(), method_id::SET),
            AccessControlEntry { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION].into(), ..Default::default() },
        ));
        // Authority
        items.push((
            AccessControlRef::new(authority::USER.nth(user_idx).unwrap().into(), method_id::SET),
            AccessControlEntry {
                acl: vec![
                    ace::AUTHORITY_SET_ENABLED,
                    ace::USER_SET_COMMON_NAME.nth(user_idx).unwrap(),
                ]
                .into(),
                ..Default::default()
            },
        ));
        // C_PIN
        items.push((
            AccessControlRef::new(c_pin::USER.nth(user_idx).unwrap().into(), method_id::GET),
            AccessControlEntry { acl: vec![ace::C_PIN_ADMINS_GET_ALL_NOPIN].into(), ..Default::default() },
        ));
        items.push((
            AccessControlRef::new(c_pin::USER.nth(user_idx).unwrap().into(), method_id::SET),
            AccessControlEntry {
                acl: vec![ace::C_PIN_USER_SET_PIN.nth(user_idx).unwrap()].into(),
                ..Default::default()
            },
        ));
    }

    // Ranges
    {
        // ACE
        items.push((
            AccessControlRef::new(ace::K_AES_256_GLOBAL_RANGE_GEN_KEY.into(), method_id::SET),
            AccessControlEntry { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION].into(), ..Default::default() },
        ));
        items.push((
            AccessControlRef::new(ace::LOCKING_GLOBAL_RANGE_GET_RANGE_START_TO_ACTIVE_KEY.into(), method_id::SET),
            AccessControlEntry { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION].into(), ..Default::default() },
        ));
        items.push((
            AccessControlRef::new(ace::LOCKING_GLOBAL_RANGE_SET_RD_LOCKED.into(), method_id::SET),
            AccessControlEntry { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION].into(), ..Default::default() },
        ));
        items.push((
            AccessControlRef::new(ace::LOCKING_GLOBAL_RANGE_SET_WR_LOCKED.into(), method_id::SET),
            AccessControlEntry { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION].into(), ..Default::default() },
        ));
        // Locking
        items.push((
            AccessControlRef::new(locking::GLOBAL_RANGE.into(), method_id::GET),
            AccessControlEntry {
                acl: vec![
                    ace::LOCKING_GLOBAL_RANGE_GET_RANGE_START_TO_ACTIVE_KEY,
                    ace::ANYBODY_GET_COMMON_NAME,
                ]
                .into(),
                ..Default::default()
            },
        ));
        items.push((
            AccessControlRef::new(locking::GLOBAL_RANGE.into(), method_id::SET),
            AccessControlEntry {
                acl: vec![
                    ace::LOCKING_GLBL_RNG_ADMINS_SET,
                    ace::LOCKING_GLOBAL_RANGE_SET_RD_LOCKED,
                    ace::LOCKING_GLOBAL_RANGE_SET_WR_LOCKED,
                    ace::ADMINS_SET_COMMON_NAME,
                ]
                .into(),
                ..Default::default()
            },
        ));
        // K_AES_256
        items.push((
            AccessControlRef::new(k_aes_256::GLOBAL_RANGE_KEY.into(), method_id::GEN_KEY),
            AccessControlEntry { acl: vec![ace::K_AES_256_GLOBAL_RANGE_GEN_KEY].into(), ..Default::default() },
        ));
    }
    for range_idx in RANGE_IDX {
        // ACE
        items.push((
            AccessControlRef::new(ace::K_AES_256_RANGE_GEN_KEY.nth(range_idx).unwrap().into(), method_id::SET),
            AccessControlEntry { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION].into(), ..Default::default() },
        ));
        items.push((
            AccessControlRef::new(
                ace::LOCKING_RANGE_GET_RANGE_START_TO_ACTIVE_KEY.nth(range_idx).unwrap().into(),
                method_id::SET,
            ),
            AccessControlEntry { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION].into(), ..Default::default() },
        ));
        items.push((
            AccessControlRef::new(ace::LOCKING_RANGE_SET_RD_LOCKED.nth(range_idx).unwrap().into(), method_id::SET),
            AccessControlEntry { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION].into(), ..Default::default() },
        ));
        items.push((
            AccessControlRef::new(ace::LOCKING_RANGE_SET_WR_LOCKED.nth(range_idx).unwrap().into(), method_id::SET),
            AccessControlEntry { acl: vec![ace::ACE_SET_BOOLEAN_EXPRESSION].into(), ..Default::default() },
        ));
        // Locking
        items.push((
            AccessControlRef::new(locking::RANGE.nth(range_idx).unwrap().into(), method_id::GET),
            AccessControlEntry {
                acl: vec![
                    ace::LOCKING_RANGE_GET_RANGE_START_TO_ACTIVE_KEY.nth(range_idx).unwrap(),
                    ace::ANYBODY_GET_COMMON_NAME,
                ]
                .into(),
                ..Default::default()
            },
        ));
        items.push((
            AccessControlRef::new(locking::RANGE.nth(range_idx).unwrap().into(), method_id::SET),
            AccessControlEntry {
                acl: vec![
                    ace::LOCKING_ADMINS_RANGE_START_TO_LOR,
                    ace::LOCKING_RANGE_SET_RD_LOCKED.nth(range_idx).unwrap(),
                    ace::LOCKING_RANGE_SET_WR_LOCKED.nth(range_idx).unwrap(),
                    ace::ADMINS_SET_COMMON_NAME,
                ]
                .into(),
                ..Default::default()
            },
        ));
        // K_AES_256
        items.push((
            AccessControlRef::new(k_aes_256::RANGE_KEY.nth(range_idx).unwrap().into(), method_id::GEN_KEY),
            AccessControlEntry {
                acl: vec![ace::K_AES_256_RANGE_GEN_KEY.nth(range_idx).unwrap()].into(),
                ..Default::default()
            },
        ));
    }

    let count = items.len();
    let out: AccessControlTable = items.into_iter().collect();
    assert_eq!(out.len(), count);
    out
}
