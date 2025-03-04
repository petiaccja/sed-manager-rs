use sed_manager::{
    applications::{is_activating_locking_supported, is_revert_supported, is_taking_ownership_supported},
    messaging::discovery::{Discovery, FeatureCode, LockingDescriptor},
};

use crate::ActivitySupport;

impl ActivitySupport {
    pub fn none() -> Self {
        Self {
            take_ownership: false,
            activate_locking: false,
            locking_ranges: false,
            users: false,
            permissions: false,
            shadow_mbr: false,
            revert: false,
            com_id_status: false,
            stack_reset: false,
        }
    }

    pub fn from_discovery(discovery: &Discovery) -> Self {
        Self {
            activate_locking: is_activating_locking_supported(discovery),
            com_id_status: false, // Always supported / TODO.
            locking_ranges: is_locking_ranges_supported(discovery),
            permissions: false, // TODO
            revert: is_revert_supported(discovery),
            shadow_mbr: false,  // TODO
            stack_reset: false, // Always supported / TODO.
            take_ownership: is_taking_ownership_supported(discovery),
            users: false, // TODO
        }
    }
}

fn is_locking_ranges_supported(discovery: &Discovery) -> bool {
    let ssc = discovery.get_primary_ssc();
    let ssc_supported = ssc.map(|ssc| ssc.feature_code() != FeatureCode::KeyPerIO).unwrap_or(false);
    let Some(locking_desc) = discovery.get::<LockingDescriptor>() else {
        return false;
    };
    ssc_supported && locking_desc.locking_enabled && locking_desc.locking_supported
}
