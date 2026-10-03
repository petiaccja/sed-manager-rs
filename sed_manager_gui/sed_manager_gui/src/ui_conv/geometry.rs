//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_manager::{Alignment, Geometry};

use crate::ui_conv::IntoUi;
use sed_manager_gui_slint as ui;

impl IntoUi for Geometry {
    type Ui = ui::Geometry;

    fn into_ui(&self) -> Self::Ui {
        ui::Geometry {
            logical_sector_count: ui::Sector { value: self.logical_sector_count as i64 },
            logical_sector_size: self.logical_sector_size as i32,
        }
    }
}
impl IntoUi for Alignment {
    type Ui = ui::Alignment;

    fn into_ui(&self) -> Self::Ui {
        ui::Alignment {
            alignment_granularity: ui::Sector { value: self.alignment_granularity as i64 },
            alignment_required: self.alignment_required,
            lowest_aligned_lba: ui::Sector { value: self.lowest_aligned_lba as i64 },
        }
    }
}
