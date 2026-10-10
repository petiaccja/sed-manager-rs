//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_manager::{Alignment, Geometry};

use crate::ui_conv::FromBackend;
use sed_manager_gui_slint as ui;

impl FromBackend<&Geometry> for ui::Geometry {
    fn from_backend(value: &Geometry) -> Self {
        ui::Geometry {
            logical_sector_count: ui::Sector { value: value.logical_sector_count as i64 },
            logical_sector_size: value.logical_sector_size as i32,
        }
    }
}
impl FromBackend<&Alignment> for ui::Alignment {
    fn from_backend(value: &Alignment) -> Self {
        ui::Alignment {
            alignment_granularity: ui::Sector { value: value.alignment_granularity as i64 },
            alignment_required: value.alignment_required,
            lowest_aligned_lba: ui::Sector { value: value.lowest_aligned_lba as i64 },
        }
    }
}
