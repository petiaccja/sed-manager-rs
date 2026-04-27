use sed_spec::{objects::SecurityProviderRef, preconfig::opal_2::admin::sp};

use crate::tper::{Admin, Locking, security_provider::SecurityProvider};

mod preconfig_admin;
mod preconfig_locking;

#[derive(Debug)]
pub struct Opal2TPer {
    pub admin: Admin,
    pub locking: Locking,
}

impl Opal2TPer {
    pub fn sp(&self, uid: SecurityProviderRef) -> Option<&dyn SecurityProvider> {
        match uid {
            sp::ADMIN => Some(&self.admin),
            sp::LOCKING => Some(&self.locking),
            _ => None,
        }
    }

    pub fn sp_mut(&mut self, uid: SecurityProviderRef) -> Option<&mut dyn SecurityProvider> {
        match uid {
            sp::ADMIN => Some(&mut self.admin),
            sp::LOCKING => Some(&mut self.locking),
            _ => None,
        }
    }
}

impl Default for Opal2TPer {
    fn default() -> Self {
        Self { admin: preconfig_admin::preconfig(), locking: preconfig_locking::preconfig() }
    }
}
