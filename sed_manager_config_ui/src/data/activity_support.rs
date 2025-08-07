//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_manager::{
    applications::{
        is_activating_locking_supported, is_change_password_supported, is_mbr_editor_supported,
        is_permission_editor_supported, is_range_editor_supported, is_revert_supported, is_taking_ownership_supported,
        is_user_editor_supported,
    },
    messaging::discovery::Discovery,
};

use crate::ActivitySupport;

impl ActivitySupport {
    pub fn none() -> Self {
        Self {
            take_ownership: false,
            activate_locking: false,
            change_password: false,
            range_editor: false,
            user_editor: false,
            access_control_editor: false,
            shadow_mbr: false,
            revert: false,
            com_id_status: false,
            stack_reset: false,
        }
    }

    pub fn from_discovery(discovery: &Discovery) -> Self {
        Self {
            activate_locking: is_activating_locking_supported(discovery),
            change_password: is_change_password_supported(),
            com_id_status: false, // Always supported | Not implemented
            range_editor: is_range_editor_supported(discovery),
            access_control_editor: is_permission_editor_supported(discovery),
            revert: is_revert_supported(discovery),
            shadow_mbr: is_mbr_editor_supported(discovery),
            stack_reset: true, // Always supported
            take_ownership: is_taking_ownership_supported(discovery),
            user_editor: is_user_editor_supported(discovery),
        }
    }
}
