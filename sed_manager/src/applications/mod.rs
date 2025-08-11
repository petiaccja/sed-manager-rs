//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

mod activate_locking;
mod change_password;
pub mod error;
mod mbr_edit_session;
mod permission_session;
mod range_edit_session;
mod revert;
mod take_ownership;
pub mod test_fixtures;
mod user_edit_session;
mod utility;

pub use activate_locking::{activate_locking, is_activating_locking_supported, verify_locking_activation};
pub use change_password::{is_change_password_supported, list_password_authorities};
pub use error::Error;
pub use mbr_edit_session::{is_mbr_editor_supported, MBREditSession};
pub use permission_session::{is_permission_editor_supported, PermissionEditSession};
pub use range_edit_session::{is_range_editor_supported, RangeEditSession};
pub use revert::{is_revert_supported, revert};
pub use take_ownership::{is_taking_ownership_supported, take_ownership, verify_ownership};
pub use user_edit_session::{is_user_editor_supported, UserEditSession};
pub use utility::{get_admin_sp, get_feature_lookup, get_general_lookup, get_locking_admins, get_locking_sp};
