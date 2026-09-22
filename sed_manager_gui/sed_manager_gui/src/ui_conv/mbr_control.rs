//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_manager_gui_slint as ui;
use sed_spec::objects::MbrControl;

use crate::ui_conv::IntoUi;

impl IntoUi for MbrControl {
    type Ui = ui::Mbr;

    fn into_ui(&self) -> Self::Ui {
        ui::Mbr { enabled: self.enable.unwrap_or(false), done: self.done.unwrap_or(false), ..Default::default() }
    }
}
