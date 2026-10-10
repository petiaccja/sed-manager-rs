//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::num::NonZero;

use sed_manager_gui_slint as ui;
use sed_packet::discovery::FeatureDescriptor;
use sed_spec::objects::{LockingRange, LockingRangeRef, SecurityProviderRef};
use slint::{SharedString, ToSharedString};

use crate::ui_conv::{FromBackend, FromBackendName};

const INVALID_LOCKING_RANGE: LockingRangeRef = LockingRangeRef::from_half(NonZero::new(0xFFFF_FFFF).unwrap());

impl FromBackendName<&LockingRange> for ui::LockingRange {
    fn from_backend_name(
        value: &LockingRange,
        features: &[FeatureDescriptor],
        sp: Option<SecurityProviderRef>,
    ) -> Self {
        let uid = value.uid.unwrap_or(INVALID_LOCKING_RANGE);
        let start = value.range_start.unwrap_or(0);
        let length = value.range_length.unwrap_or(0);
        ui::LockingRange {
            common_name: value.common_name.as_deref().unwrap_or("").to_shared_string(),
            name: SharedString::from_backend_name(&uid, features, sp),
            uid: ui::Uid::from_backend(&uid),
            start: ui::Sector { value: start.cast_signed() },
            end: ui::Sector { value: start.wrapping_add(length).cast_signed() },
            read_lock_enabled: value.read_lock_enabled.unwrap_or(false),
            write_lock_enabled: value.write_lock_enabled.unwrap_or(false),
            read_locked: value.read_locked.unwrap_or(false),
            write_locked: value.write_locked.unwrap_or(false),
            // Erase is an action, not a persisted column. It's only ever set
            // by the user to request an erase on commit.
            erase: false,
        }
    }
}
