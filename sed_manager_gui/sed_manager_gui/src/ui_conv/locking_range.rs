//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::num::NonZero;

use sed_manager_gui_slint as ui;
use sed_packet::discovery::FeatureDescriptor;
use sed_spec::objects::{LockingRange, LockingRangeRef, SecurityProviderRef};
use slint::{SharedString, ToSharedString};

use crate::ui_conv::{ConversionError, FromBackend, FromBackendName, TryFromUi, TryIntoBackend};

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

impl TryFromUi<&ui::LockingRange> for LockingRange {
    type Error = ConversionError;

    fn try_from_ui(value: &ui::LockingRange) -> Result<Self, Self::Error> {
        Ok(Self {
            uid: Some((&value.uid).try_into_backend()?),
            name: None, // The name of the UI object is based on the UID, not on this field.
            common_name: Some(value.common_name.to_string()),
            range_start: Some(value.start.value as u64),
            range_length: Some((value.end.value - value.start.value) as u64),
            read_lock_enabled: Some(value.read_lock_enabled),
            write_lock_enabled: Some(value.write_lock_enabled),
            read_locked: Some(value.read_locked),
            write_locked: Some(value.write_locked),
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use sed_spec::preconfig::opal_2;

    use super::*;

    // A sanity check for accidentally mixing up all the booleans and the length
    // conversion logic.
    #[test]
    fn roundtrip() {
        let locking_range = LockingRange {
            uid: Some(opal_2::locking::locking::GLOBAL_RANGE),
            common_name: Some("Global range".into()),
            range_start: Some(1234),
            range_length: Some(5678),
            read_lock_enabled: Some(true),
            write_lock_enabled: Some(false),
            read_locked: Some(false),
            write_locked: Some(true),
            ..Default::default()
        };

        let ui_locking_range = ui::LockingRange::from_backend(&locking_range);
        let roundtrip = LockingRange::try_from_ui(&ui_locking_range).unwrap();
        assert_eq!(locking_range, roundtrip);
    }
}
