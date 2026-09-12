//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::num::NonZero;

use sed_manager_gui_slint as ui;
use sed_packet::discovery::FeatureDescriptor;
use sed_spec::objects::{LockingRange, LockingRangeRef, SecurityProviderRef};
use slint::ToSharedString;

use crate::ui_conv::{IntoUi, IntoUiName};

const INVALID_LOCKING_RANGE: LockingRangeRef = LockingRangeRef::from_half(NonZero::new(0xFFFF_FFFF).unwrap());

impl IntoUi for LockingRange {
    type Ui = ui::LockingRange;

    fn into_ui(&self) -> Self::Ui {
        let start = self.range_start.unwrap_or(0);
        let length = self.range_length.unwrap_or(0);
        ui::LockingRange {
            common_name: self.common_name.as_deref().unwrap_or("").to_shared_string(),
            name: self.uid.unwrap_or(INVALID_LOCKING_RANGE).to_shared_string(),
            uid: self.uid.unwrap_or(INVALID_LOCKING_RANGE).into_ui(),
            start: ui::Sector { value: start.cast_signed() },
            end: ui::Sector { value: start.wrapping_add(length).cast_signed() },
            read_lock_enabled: self.read_lock_enabled.unwrap_or(false),
            write_lock_enabled: self.write_lock_enabled.unwrap_or(false),
            read_locked: self.read_locked.unwrap_or(false),
            write_locked: self.write_locked.unwrap_or(false),
            // Erase is an action, not a persisted column. It's only ever set
            // by the user to request an erase on commit.
            erase: false,
        }
    }
}

impl IntoUiName for LockingRange {
    type Ui = ui::LockingRange;

    fn into_ui_name(&self, features: &[FeatureDescriptor], sp: Option<SecurityProviderRef>) -> Self::Ui {
        ui::LockingRange {
            name: self.uid.unwrap_or(INVALID_LOCKING_RANGE).into_ui_name(features, sp),
            ..self.into_ui()
        }
    }
}
