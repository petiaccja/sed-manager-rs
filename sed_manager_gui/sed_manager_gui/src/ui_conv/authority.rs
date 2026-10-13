//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::{num::NonZero, rc::Rc};

use sed_manager_gui_slint as ui;
use sed_packet::discovery::FeatureDescriptor;
use sed_spec::objects::{Authority, AuthorityRef, SecurityProviderRef};
use slint::{SharedString, ToSharedString, VecModel};

use crate::ui_conv::{FromBackend, FromBackendName};

const INVALID_AUTHORITY: AuthorityRef = AuthorityRef::from_half(NonZero::new(0xFFFF_FFFF).unwrap());

impl FromBackendName<&Authority> for ui::Authority {
    fn from_backend_name(value: &Authority, features: &[FeatureDescriptor], sp: Option<SecurityProviderRef>) -> Self {
        let uid = value.uid.unwrap_or(INVALID_AUTHORITY);
        ui::Authority {
            common_name: value.common_name.as_deref().unwrap_or("").to_shared_string(),
            // Assume enabled. This will give us an error from the TPer when
            // trying to modify the authority, which is better than hiding the authority
            // from the UI.
            enabled: value.enabled.unwrap_or(true),
            // Assume NOT a class. This will give us an error from the TPer if
            // we try to authenticate, but it's better than hiding it from the
            // UI.
            is_class: value.is_class.unwrap_or(false),
            locking_range_access: Rc::from(VecModel::from(Vec::new())).into(),
            mbr_access: ui::MbrAccess::default(),
            name: SharedString::from_backend_name(&uid, features, sp),
            uid: ui::Uid::from_backend(&uid),
        }
    }
}
