//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_manager::{Alignment, Geometry};
use sed_manager_gui_slint as ui;

use crate::ui_conv::{FromBackend, FromUi};

impl FromBackend<&Geometry> for ui::Geometry {
    fn from_backend(value: &Geometry) -> Self {
        Self {
            logical_sector_count: ui::Sector { value: value.logical_sector_count as i64 },
            logical_sector_size: value.logical_sector_size as i32,
        }
    }
}

impl FromBackend<&Alignment> for ui::Alignment {
    fn from_backend(value: &Alignment) -> Self {
        Self {
            alignment_granularity: ui::Sector { value: value.alignment_granularity as i64 },
            alignment_required: value.alignment_required,
            lowest_aligned_lba: ui::Sector { value: value.lowest_aligned_lba as i64 },
        }
    }
}

impl FromUi<&ui::Alignment> for Alignment {
    fn from_ui(value: &ui::Alignment) -> Self {
        Self {
            alignment_required: value.alignment_required,
            alignment_granularity: value.alignment_granularity.value.max(0) as u64,
            lowest_aligned_lba: value.lowest_aligned_lba.value.max(0) as u64,
        }
    }
}
