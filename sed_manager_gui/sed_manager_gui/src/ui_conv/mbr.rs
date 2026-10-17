//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_manager_gui_slint as ui;
use sed_spec::{objects::MbrControl, preconfig::core::shared::mbr_control};

use crate::ui_conv::{FromBackend, FromUi};

/// A helper structure with all MBR-related parameters relevant to the UI. The
/// parameters come from different sources (L0 discovery, Table table,
/// MbrControl table), that's why this helper is needed.
pub struct MbrDesc {
    pub supported: bool,
    /// The size of the MBR table in logical sectors.
    pub size: Option<u32>,
    pub control: Option<MbrControl>,
}

impl FromBackend<&MbrDesc> for ui::Mbr {
    fn from_backend(value: &MbrDesc) -> Self {
        let control = value.control.as_ref();
        ui::Mbr {
            supported: value.supported,
            size: ui::Sector { value: value.size.unwrap_or(0) as i64 },
            enabled: control.and_then(|control| control.enable).unwrap_or(false),
            done: control.and_then(|control| control.done).unwrap_or(false),
            ..Default::default()
        }
    }
}

impl FromUi<&ui::Mbr> for MbrControl {
    fn from_ui(value: &ui::Mbr) -> Self {
        Self {
            uid: Some(mbr_control::MBR_CONTROL),
            enable: Some(value.enabled),
            done: Some(value.done),
            done_on_reset: None,
        }
    }
}
