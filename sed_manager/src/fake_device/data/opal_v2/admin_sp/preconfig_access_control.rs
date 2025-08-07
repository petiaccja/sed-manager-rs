//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::fake_device::data::access_control_table::{AccessControlEntry, AccessControlRef, AccessControlTable};
use crate::spec::invoking_id;
use crate::spec::opal::admin::*;

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
            AccessControlEntry { acl: vec![ace::ANYBODY].into(), ..Default::default() },
        ),
        // Authority
        (
            AccessControlRef::new(table_id::AUTHORITY.into(), method_id::NEXT),
            AccessControlEntry { acl: vec![ace::ANYBODY].into(), ..Default::default() },
        ),
        (
            AccessControlRef::new(table_id::AUTHORITY.into(), method_id::GET),
            AccessControlEntry { acl: vec![ace::ANYBODY].into(), ..Default::default() },
        ),
        // C_PIN
        (
            AccessControlRef::new(table_id::C_PIN.into(), method_id::NEXT),
            AccessControlEntry { acl: vec![ace::ANYBODY].into(), ..Default::default() },
        ),
        (
            AccessControlRef::new(c_pin::SID.into(), method_id::GET),
            AccessControlEntry { acl: vec![ace::C_PIN_SID_GET_NOPIN].into(), ..Default::default() },
        ),
        (
            AccessControlRef::new(c_pin::SID.into(), method_id::SET),
            AccessControlEntry { acl: vec![ace::C_PIN_SID_SET_PIN].into(), ..Default::default() },
        ),
        (
            AccessControlRef::new(c_pin::MSID.into(), method_id::GET),
            AccessControlEntry { acl: vec![ace::C_PIN_MSID_GET_PIN].into(), ..Default::default() },
        ),
        // SP
        (
            AccessControlRef::new(invoking_id::THIS_SP.into(), method_id::AUTHENTICATE),
            AccessControlEntry { acl: vec![ace::ANYBODY].into(), ..Default::default() },
        ),
        (
            AccessControlRef::new(invoking_id::THIS_SP.into(), method_id::RANDOM),
            AccessControlEntry { acl: vec![ace::ANYBODY].into(), ..Default::default() },
        ),
        (
            AccessControlRef::new(table_id::SP.into(), method_id::NEXT),
            AccessControlEntry { acl: vec![ace::ANYBODY].into(), ..Default::default() },
        ),
        (
            AccessControlRef::new(table_id::SP.into(), method_id::GET),
            AccessControlEntry { acl: vec![ace::ANYBODY].into(), ..Default::default() },
        ),
        (
            AccessControlRef::new(table_id::SP.into(), method_id::REVERT),
            AccessControlEntry { acl: vec![ace::SP_SID, ace::ADMIN].into(), ..Default::default() },
        ),
        (
            AccessControlRef::new(table_id::SP.into(), method_id::ACTIVATE),
            AccessControlEntry { acl: vec![ace::SP_SID].into(), ..Default::default() },
        ),
    ];

    // Admins
    for admin_idx in 1..4 {
        // Authority
        items.push((
            AccessControlRef::new(authority::ADMIN.nth(admin_idx).unwrap().as_uid(), method_id::SET),
            AccessControlEntry { acl: vec![ace::SET_ENABLED].into(), ..Default::default() },
        ));
        items.push((
            AccessControlRef::new(c_pin::ADMIN.nth(admin_idx).unwrap().as_uid(), method_id::GET),
            AccessControlEntry { acl: vec![ace::C_PIN_SID_GET_NOPIN].into(), ..Default::default() },
        ));
        items.push((
            AccessControlRef::new(c_pin::ADMIN.nth(admin_idx).unwrap().as_uid(), method_id::SET),
            AccessControlEntry { acl: vec![ace::C_PIN_ADMINS_SET_PIN].into(), ..Default::default() },
        ));
    }

    let count = items.len();
    let access_control_table: AccessControlTable = items.into_iter().collect();
    assert_eq!(access_control_table.len(), count);
    access_control_table
}
