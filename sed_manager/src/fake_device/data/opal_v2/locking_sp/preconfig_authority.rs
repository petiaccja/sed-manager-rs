//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::fake_device::data::object_table::AuthorityTable;
use crate::spec::column_types::{AuthMethod, CredentialRef};
use crate::spec::objects::Authority;
use crate::spec::opal::locking::*;

use super::{ADMIN_IDX, USER_IDX};

pub fn preconfig_authority() -> AuthorityTable {
    let mut items = vec![
        Authority { uid: authority::ANYBODY, name: "Anybody".into(), is_class: false, ..Default::default() },
        Authority {
            uid: authority::ADMINS,
            name: "Admins".into(),
            is_class: true,
            enabled: true,
            ..Default::default()
        },
        Authority { uid: authority::USERS, name: "Users".into(), is_class: true, enabled: true, ..Default::default() },
    ];

    for index in ADMIN_IDX {
        items.push(Authority {
            uid: authority::ADMIN.nth(index).unwrap(),
            name: format!("Admin{}", index).into(),
            is_class: false,
            class: authority::ADMINS,
            enabled: (index == 1),
            operation: AuthMethod::Password,
            credential: CredentialRef::new_other(c_pin::ADMIN.nth(index).unwrap()),
            ..Default::default()
        });
    }

    for index in USER_IDX {
        items.push(Authority {
            uid: authority::USER.nth(index).unwrap(),
            name: format!("User{}", index).into(),
            is_class: false,
            class: authority::USERS,
            enabled: false,
            operation: AuthMethod::Password,
            credential: CredentialRef::new_other(c_pin::USER.nth(index).unwrap()),
            ..Default::default()
        });
    }

    items.into_iter().collect()
}
