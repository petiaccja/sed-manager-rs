//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::spec::opal::admin::*;
use crate::{
    fake_device::data::object_table::ACETable,
    spec::objects::{ace::ace_expr, Authority, ACE, CPIN},
};

macro_rules! all_columns {
    () => {
        (0..32).into_iter().collect()
    };
}

pub fn preconfig_ace() -> ACETable {
    let items = [
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
        // Authority table
        ACE {
            uid: ace::SET_ENABLED,
            boolean_expr: ace_expr!((authority::SID)),
            columns: [Authority::ENABLED].into(),
            ..Default::default()
        },
        // C_PIN table
        ACE {
            uid: ace::C_PIN_SID_GET_NOPIN,
            boolean_expr: ace_expr!((authority::ADMINS) (authority::SID) ||),
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
            uid: ace::C_PIN_SID_SET_PIN,
            boolean_expr: ace_expr!((authority::SID)),
            columns: [CPIN::PIN].into(),
            ..Default::default()
        },
        ACE {
            uid: ace::C_PIN_MSID_GET_PIN,
            boolean_expr: ace_expr!((authority::ANYBODY)),
            columns: [CPIN::UID, CPIN::PIN].into(),
            ..Default::default()
        },
        ACE {
            uid: ace::C_PIN_ADMINS_SET_PIN,
            boolean_expr: ace_expr!((authority::ADMINS) (authority::SID) ||),
            columns: [CPIN::PIN].into(),
            ..Default::default()
        },
        // SP
        ACE {
            uid: ace::SP_SID,
            boolean_expr: ace_expr!((authority::SID)),
            columns: all_columns!(),
            ..Default::default()
        },
    ];

    items.into_iter().collect()
}
