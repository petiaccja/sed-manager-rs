use sed_spec::{methods::MethodStatus, objects::SecurityProviderRef, preconfig::opal_2::admin::sp};

use crate::tper::{Admin, Locking, security_provider::SecurityProvider};

mod preconfig_admin;
mod preconfig_locking;

#[derive(Debug)]
pub struct Opal2TPer {
    admin: Admin,
    locking: Locking,
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

    pub fn admin_sp(&self) -> &Admin {
        &self.admin
    }

    pub fn admin_sp_mut(&mut self) -> &mut Admin {
        &mut self.admin
    }

    pub fn locking_sp(&self) -> Option<&Locking> {
        Some(&self.locking)
    }

    #[allow(unused)]
    pub fn locking_sp_mut(&mut self) -> Option<&mut Locking> {
        Some(&mut self.locking)
    }

    pub fn restore_preconfig(&mut self, sp: SecurityProviderRef) -> Result<(), MethodStatus> {
        match sp {
            sp::LOCKING => {
                self.locking = preconfig_locking::preconfig();
                Ok(())
            }
            _ => Err(MethodStatus::InvalidParameter),
        }
    }
}

impl Default for Opal2TPer {
    fn default() -> Self {
        Self { admin: preconfig_admin::preconfig(), locking: preconfig_locking::preconfig() }
    }
}
