//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::num::NonZero;

use sed_manager_gui_slint as ui;
use sed_packet::discovery::FeatureDescriptor;
use sed_spec::objects::{Authority, AuthorityRef, CPinRef, SecurityProviderRef};
use sed_spec::preconfig::opal_2::{admin, locking};
use slint::{SharedString, ToSharedString};

use crate::ui_conv::{FromBackend, FromBackendName};

const INVALID_AUTHORITY: AuthorityRef = AuthorityRef::from_half(NonZero::new(0xFFFF_FFFF).unwrap());
const ADMIN_CLASSES: [AuthorityRef; 2] = [admin::authority::ADMINS, locking::authority::ADMINS];

impl FromBackendName<&Authority> for ui::Authority {
    fn from_backend_name(value: &Authority, features: &[FeatureDescriptor], sp: Option<SecurityProviderRef>) -> Self {
        let uid = value.uid.unwrap_or(INVALID_AUTHORITY);
        ui::Authority {
            uid: ui::Uid::from_backend(&uid),
            name: SharedString::from_backend_name(&uid, features, sp),
            common_name: value.common_name.as_deref().unwrap_or("").to_shared_string(),
            // Assume enabled. This will give us an error from the TPer when
            // trying to modify the authority, which is better than hiding the authority
            // from the UI.
            enabled: value.enabled.unwrap_or(true),
            // Assume NOT a class. This will give us an error from the TPer if
            // we try to authenticate, but it's better than hiding it from the
            // UI.
            is_class: value.is_class.unwrap_or(false),
            has_password: value.credential.is_some_and(|cred| CPinRef::try_from(cred).is_ok()),
            is_admin: value.class.is_some_and(|class| ADMIN_CLASSES.contains(&class)),
        }
    }
}
