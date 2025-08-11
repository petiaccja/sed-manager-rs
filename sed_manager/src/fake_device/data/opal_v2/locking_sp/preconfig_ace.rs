//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::fake_device::data::object_table::ACETable;
use crate::spec::objects::{ace::ace_expr, ACE};
use crate::spec::objects::{Authority, LockingRange, MBRControl, CPIN, KAES256};
use crate::spec::opal::locking::*;

use super::{RANGE_IDX, USER_IDX};

macro_rules! all_columns {
    () => {
        (0..32).into_iter().collect()
    };
}

pub fn preconfig_ace() -> ACETable {
    let mut items = vec![
        // Base ACEs
        ACE {
            uid: ace::ANYBODY,
            boolean_expr: ace_expr!((authority::ANYBODY)),
            columns: all_columns!(),
            ..Default::default()
        },
        ACE {
            uid: ace::ADMIN,
            boolean_expr: ace_expr!((authority::ADMINS)),
            columns: all_columns!(),
            ..Default::default()
        },
        ACE {
            uid: ace::ANYBODY_GET_COMMON_NAME,
            boolean_expr: ace_expr!((authority::ANYBODY)),
            columns: [0, 2].into(), // UID, CommonName
            ..Default::default()
        },
        ACE {
            uid: ace::ADMINS_SET_COMMON_NAME,
            boolean_expr: ace_expr!((authority::ADMINS)),
            columns: [2].into(), // CommonName
            ..Default::default()
        },
        // ACE
        ACE {
            uid: ace::ACE_GET_ALL,
            boolean_expr: ace_expr!((authority::ADMINS)),
            columns: all_columns!(),
            ..Default::default()
        },
        ACE {
            uid: ace::ACE_SET_BOOLEAN_EXPRESSION,
            boolean_expr: ace_expr!((authority::ADMINS)),
            columns: [ACE::BOOLEAN_EXPR].into(),
            ..Default::default()
        },
        // Authority
        ACE {
            uid: ace::AUTHORITY_GET_ALL,
            boolean_expr: ace_expr!((authority::ADMINS)),
            columns: all_columns!(),
            ..Default::default()
        },
        ACE {
            uid: ace::AUTHORITY_SET_ENABLED,
            boolean_expr: ace_expr!((authority::ADMINS)),
            columns: [Authority::ENABLED].into(),
            ..Default::default()
        },
        // ... Users ...
        // C_PIN
        ACE {
            uid: ace::C_PIN_ADMINS_GET_ALL_NOPIN,
            boolean_expr: ace_expr!((authority::ADMINS)),
            columns: [
                CPIN::UID,
                CPIN::CHAR_SET,
                CPIN::TRY_LIMIT,
                CPIN::TRIES,
                CPIN::PERSISTENCE,
            ]
            .into(),
            ..Default::default()
        },
        ACE {
            uid: ace::C_PIN_ADMINS_SET_PIN,
            boolean_expr: ace_expr!((authority::ADMINS)),
            columns: [CPIN::PIN].into(),
            ..Default::default()
        },
        // ... Users ...
        // K_AES_*
        ACE {
            uid: ace::K_AES_MODE,
            boolean_expr: ace_expr!((authority::ANYBODY)),
            columns: [KAES256::MODE].into(),
            ..Default::default()
        },
        // Locking
        ACE {
            uid: ace::LOCKING_ADMINS_RANGE_START_TO_LOR,
            boolean_expr: ace_expr!((authority::ADMINS)),
            columns: (LockingRange::RANGE_START..LockingRange::LOCK_ON_RESET).into_iter().collect(),
            ..Default::default()
        },
        // MBRControl
        ACE {
            uid: ace::MBR_CONTROL_ADMINS_SET,
            boolean_expr: ace_expr!((authority::ADMINS)),
            columns: [
                MBRControl::ENABLE,
                MBRControl::DONE,
                MBRControl::DONE_ON_RESET,
            ]
            .into(),
            ..Default::default()
        },
        ACE {
            uid: ace::MBR_CONTROL_SET_DONE_TO_DOR,
            boolean_expr: ace_expr!((authority::ADMINS)),
            columns: [MBRControl::DONE, MBRControl::DONE_ON_RESET].into(),
            ..Default::default()
        },
        // DataStore
        ACE {
            uid: ace::DATA_STORE_GET_ALL,
            boolean_expr: ace_expr!((authority::ADMINS)),
            columns: all_columns!(),
            ..Default::default()
        },
        ACE {
            uid: ace::DATA_STORE_SET_ALL,
            boolean_expr: ace_expr!((authority::ADMINS)),
            columns: all_columns!(),
            ..Default::default()
        },
    ];

    // Users
    for user_idx in USER_IDX {
        // Authority
        items.push(ACE {
            uid: ace::USER_SET_COMMON_NAME.nth(user_idx).unwrap(),
            boolean_expr: ace_expr!((authority::ADMINS)),
            columns: [Authority::COMMON_NAME].into(),
            ..Default::default()
        });
        // C_PIN
        items.push(ACE {
            uid: ace::C_PIN_USER_SET_PIN.nth(user_idx).unwrap(),
            boolean_expr: ace_expr!((authority::ADMINS)),
            columns: [CPIN::PIN].into(),
            ..Default::default()
        });
    }

    // Ranges
    let range_start_to_active_key = LockingRange::RANGE_START..=LockingRange::ACTIVE_KEY;
    let range_start_to_lor = LockingRange::RANGE_START..=LockingRange::LOCK_ON_RESET;
    let range_admins_set = LockingRange::READ_LOCK_ENABLED..=LockingRange::LOCK_ON_RESET;
    {
        // K_AES_256
        items.push(ACE {
            uid: ace::K_AES_256_GLOBAL_RANGE_GEN_KEY,
            boolean_expr: ace_expr!((authority::ADMINS)),
            columns: all_columns!(),
            ..Default::default()
        });
        // Locking
        items.push(ACE {
            uid: ace::LOCKING_GLOBAL_RANGE_GET_RANGE_START_TO_ACTIVE_KEY,
            boolean_expr: ace_expr!((authority::ADMINS)),
            columns: range_start_to_active_key.clone().into_iter().collect(),
            ..Default::default()
        });
        items.push(ACE {
            uid: ace::LOCKING_GLOBAL_RANGE_SET_RD_LOCKED,
            boolean_expr: ace_expr!((authority::ADMINS)),
            columns: [LockingRange::READ_LOCKED].into(),
            ..Default::default()
        });
        items.push(ACE {
            uid: ace::LOCKING_GLOBAL_RANGE_SET_WR_LOCKED,
            boolean_expr: ace_expr!((authority::ADMINS)),
            columns: [LockingRange::WRITE_LOCKED].into(),
            ..Default::default()
        });
        items.push(ACE {
            uid: ace::LOCKING_GLBL_RNG_ADMINS_SET,
            boolean_expr: ace_expr!((authority::ADMINS)),
            columns: range_admins_set.clone().into_iter().collect(),
            ..Default::default()
        });
    }
    for range_idx in RANGE_IDX {
        // K_AES_256
        items.push(ACE {
            uid: ace::K_AES_256_RANGE_GEN_KEY.nth(range_idx).unwrap(),
            boolean_expr: ace_expr!((authority::ADMINS)),
            columns: all_columns!(),
            ..Default::default()
        });
        // Locking
        items.push(ACE {
            uid: ace::LOCKING_RANGE_GET_RANGE_START_TO_ACTIVE_KEY.nth(range_idx).unwrap(),
            boolean_expr: ace_expr!((authority::ADMINS)),
            columns: range_start_to_active_key.clone().into_iter().collect(),
            ..Default::default()
        });
        items.push(ACE {
            uid: ace::LOCKING_RANGE_SET_RD_LOCKED.nth(range_idx).unwrap(),
            boolean_expr: ace_expr!((authority::ADMINS)),
            columns: [LockingRange::READ_LOCKED].into(),
            ..Default::default()
        });
        items.push(ACE {
            uid: ace::LOCKING_RANGE_SET_WR_LOCKED.nth(range_idx).unwrap(),
            boolean_expr: ace_expr!((authority::ADMINS)),
            columns: [LockingRange::WRITE_LOCKED].into(),
            ..Default::default()
        });
        items.push(ACE {
            uid: ace::LOCKING_ADMINS_RANGE_START_TO_LOR,
            boolean_expr: ace_expr!((authority::ADMINS)),
            columns: range_start_to_lor.clone().into_iter().collect(),
            ..Default::default()
        });
    }

    items.into_iter().collect()
}
