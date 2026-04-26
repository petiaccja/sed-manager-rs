use crate::tper::{Admin, Locking};

mod preconfig_admin;
mod preconfig_locking;

pub struct Opal2TPer {
    admin: Admin,
    locking: Locking,
}

impl Default for Opal2TPer {
    fn default() -> Self {
        Self { admin: preconfig_admin::preconfig(), locking: preconfig_locking::preconfig() }
    }
}
