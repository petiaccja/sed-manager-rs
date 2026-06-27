use std::{num::NonZero, rc::Rc};

use sed_manager_gui_slint as ui;
use sed_packet::discovery::FeatureDescriptor;
use sed_spec::objects::{Authority, AuthorityRef, SecurityProviderRef};
use slint::{ToSharedString, VecModel};

use crate::display_ui::{DisplayUi, DisplayUiName};

const INVALID_AUTHORITY: AuthorityRef = AuthorityRef::from_half(NonZero::new(0xFFFF_FFFF).unwrap());

impl DisplayUi for Authority {
    type Ui = ui::User;

    fn display_ui(&self) -> Self::Ui {
        ui::User {
            common_name: self.common_name.as_deref().unwrap_or("").to_shared_string(),
            // Assume enabled. This will give us an error from the TPer when
            // trying to modify the user, which is better than hiding the user
            // from the UI.
            enabled: self.enabled.unwrap_or(true),
            // Assume NOT a class. This will give us an error from the TPer if
            // we try to authenticate, but it's better than hiding it from the
            // UI.
            is_class: self.is_class.unwrap_or(false),
            locking_range_access: Rc::from(VecModel::from(Vec::new())).into(),
            mbr_access: ui::MbrAccess::default(),
            name: self.uid.unwrap_or(INVALID_AUTHORITY).to_shared_string(),
            uid: self.uid.unwrap_or(INVALID_AUTHORITY).display_ui(),
        }
    }
}

impl DisplayUiName for Authority {
    type Ui = ui::User;

    fn display_ui_name(&self, features: &[FeatureDescriptor], sp: Option<SecurityProviderRef>) -> Self::Ui {
        ui::User { name: self.uid.unwrap_or(INVALID_AUTHORITY).display_ui_name(features, sp), ..self.display_ui() }
    }
}
