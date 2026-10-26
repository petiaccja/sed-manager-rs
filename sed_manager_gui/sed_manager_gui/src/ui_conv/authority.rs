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

use crate::ui_conv::{ConversionError, FromBackend, FromBackendName, TryFromUi, TryIntoBackend as _};

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

impl TryFromUi<&ui::Authority> for Authority {
    type Error = ConversionError;

    fn try_from_ui(value: &ui::Authority) -> Result<Self, Self::Error> {
        Ok(Self {
            uid: Some((&value.uid).try_into_backend()?),
            name: None, // The name of the UI object is based on the UID, not on this field.
            common_name: Some(value.common_name.to_string()),
            enabled: Some(value.enabled),
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use googletest::assert_that;
    use googletest::matchers::*;
    use sed_spec::preconfig::opal_2;

    use super::*;

    // A sanity check for accidentally mixing up all the booleans and the length
    // conversion logic.
    #[test]
    fn roundtrip() {
        let authority = Authority {
            uid: opal_2::locking::authority::ADMIN.get(2),
            common_name: Some("Global range".into()),
            enabled: Some(false),
            ..Default::default()
        };

        let ui_authority = ui::Authority::from_backend(&authority);
        let roundtrip = Authority::try_from_ui(&ui_authority).unwrap();
        assert_that!(roundtrip, eq(&authority));
    }
}
