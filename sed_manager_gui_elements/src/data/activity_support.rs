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
            com_id_status: false, // Always supported | Not implemented.
            locking_ranges: is_range_config_supported(discovery),
            permissions: false, // Not implemented.
            revert: is_revert_supported(discovery),
            shadow_mbr: false, // Not implemented.
            stack_reset: true,
            take_ownership: is_taking_ownership_supported(discovery),
            users: is_user_config_supported(discovery),
        }
    }
}

fn is_range_config_supported(discovery: &Discovery) -> bool {
    // KPIO uses a different system.
    // Enterprise has no Admin authority on the Locking SP, only a dedicated authority per range.
    const SUPPORTED_SSCS: [FeatureCode; 6] = [
        FeatureCode::OpalV1,
        FeatureCode::OpalV2,
        FeatureCode::Opalite,
        FeatureCode::PyriteV1,
        FeatureCode::PyriteV2,
        FeatureCode::Ruby,
    ];
    let Some(ssc) = discovery.get_primary_ssc() else {
        return false;
    };
    let Some(locking_desc) = discovery.get::<LockingDescriptor>() else {
        return false;
    };
    SUPPORTED_SSCS.contains(&ssc.feature_code()) && locking_desc.locking_enabled && locking_desc.locking_supported
}

fn is_user_config_supported(discovery: &Discovery) -> bool {
    // Enterprise does not allow changing authorities, aside from their password.
    const SUPPORTED_SSCS: [FeatureCode; 7] = [
        FeatureCode::KeyPerIO, // Only has Admin{n}, but configuration is the same.
        FeatureCode::OpalV1,
        FeatureCode::OpalV2,
        FeatureCode::Opalite,
        FeatureCode::PyriteV1,
        FeatureCode::PyriteV2,
        FeatureCode::Ruby,
    ];
    let Some(ssc) = discovery.get_primary_ssc() else {
        return false;
    };
    let Some(locking_desc) = discovery.get::<LockingDescriptor>() else {
        return false;
    };
    SUPPORTED_SSCS.contains(&ssc.feature_code()) && locking_desc.locking_enabled && locking_desc.locking_supported
}
