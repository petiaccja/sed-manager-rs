use crate::fake_device::data::object_table::AuthorityTable;
use crate::spec;
use crate::spec::column_types::{AuthMethod, CredentialRef};
use crate::spec::objects::Authority;
use crate::spec::opal::admin::*;

use super::ADMIN_IDX;

pub fn preconfig_authority() -> AuthorityTable {
    let mut items = vec![
        Authority { uid: authority::ANYBODY, name: "Anybody".into(), ..Default::default() },
        Authority { uid: authority::ADMINS, name: "Admins".into(), ..Default::default() },
        Authority { uid: authority::MAKERS, name: "Makers".into(), ..Default::default() },
        Authority {
            uid: authority::SID,
            name: "SID".into(),
            operation: AuthMethod::Password,
            credential: CredentialRef::new_other(c_pin::SID),
            ..Default::default()
        },
        Authority {
            uid: spec::psid::admin::authority::PSID,
            name: "PSID".into(),
            operation: AuthMethod::Password.into(),
            credential: CredentialRef::new_other(spec::psid::admin::c_pin::PSID),
            ..Default::default()
        },
    ];

    for admin_idx in ADMIN_IDX {
        items.push(Authority {
            uid: authority::ADMIN.nth(admin_idx).unwrap(),
            name: format!("Admin{}", admin_idx).into(),
            enabled: false,
            operation: AuthMethod::Password,
            credential: CredentialRef::new_other(c_pin::ADMIN.nth(admin_idx).unwrap()),
            ..Default::default()
        });
    }

    items.into_iter().collect()
}
