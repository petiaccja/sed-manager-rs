use sed_manager_gui_slint as ui;
use sed_packet::{ObjectRef, Uid, discovery::FeatureDescriptor};
use sed_spec::{objects::SecurityProviderRef, path::Path, preconfig::GLOBAL_LOOKUP};
use slint::SharedString;

use crate::display_ui::{DisplayUi, DisplayUiName};

impl DisplayUi for Uid {
    type Ui = ui::Uid;

    fn display_ui(&self) -> Self::Ui {
        ui::Uid { value: self.to_u64().cast_signed() }
    }
}

impl<const TABLE: u64> DisplayUi for ObjectRef<TABLE> {
    type Ui = ui::Uid;

    fn display_ui(&self) -> Self::Ui {
        self.to_uid().display_ui()
    }
}

impl DisplayUiName for Uid {
    type Ui = SharedString;

    fn display_ui_name(&self, features: &[FeatureDescriptor], sp: Option<SecurityProviderRef>) -> Self::Ui {
        GLOBAL_LOOKUP
            .by_uid(features, *self, sp)
            .map(|name| Path::new(&name).object().to_owned())
            .unwrap_or_else(|| self.to_string())
            .into()
    }
}

impl<const TABLE: u64> DisplayUiName for ObjectRef<TABLE> {
    type Ui = SharedString;

    fn display_ui_name(&self, features: &[FeatureDescriptor], sp: Option<SecurityProviderRef>) -> Self::Ui {
        self.to_uid().display_ui_name(features, sp)
    }
}
