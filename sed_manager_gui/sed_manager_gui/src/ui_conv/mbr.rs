//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_manager_gui_slint as ui;
use sed_spec::objects::MbrControl;

use crate::ui_conv::IntoUi;

/// A helper structure with all MBR-related parameters relevant to the UI. The
/// parameters come from different sources (L0 discovery, Table table,
/// MbrControl table), that's why this helper is needed.
pub struct MbrDesc {
    pub supported: bool,
    pub size: Option<u32>,
    pub control: Option<MbrControl>,
}

impl IntoUi for MbrDesc {
    type Ui = ui::Mbr;

    fn into_ui(&self) -> Self::Ui {
        let control = self.control.as_ref();
        ui::Mbr {
            supported: self.supported,
            size: ui::Sector { value: self.size.unwrap_or(0) as i64 },
            enabled: control.and_then(|control| control.enable).unwrap_or(false),
            done: control.and_then(|control| control.done).unwrap_or(false),
            ..Default::default()
        }
    }
}
