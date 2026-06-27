use sed_manager_gui_slint as ui;
use sed_packet::{ObjectRef, Uid};

use crate::display_ui::DisplayUi;

impl DisplayUi for Uid {
    type Ui = ui::Uid;

    fn display_ui(&self) -> Self::Ui {
        ui::Uid { value: self.to_u64().cast_signed() }
    }
}

impl<const TABLE: u64> DisplayUi for ObjectRef<TABLE> {
    type Ui = ui::Uid;

    fn display_ui(&self) -> Self::Ui {
        ui::Uid { value: self.to_u64().cast_signed() }
    }
}
