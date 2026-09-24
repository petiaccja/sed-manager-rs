//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_spec::{
    methods::MethodStatus, objects::SecurityProviderRef, preconfig::enterprise::admin::sp, types::LifeCycleState,
};

use crate::tper::{Admin, Locking, security_provider::SecurityProvider};

mod preconfig_admin;
mod preconfig_locking;

#[derive(Debug)]
pub struct EnterpriseTper {
    admin: Admin,
    locking: Locking,
}

impl EnterpriseTper {
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

    pub fn restore_preconfig(&mut self, sp: SecurityProviderRef) -> Result<Vec<SecurityProviderRef>, MethodStatus> {
        match sp {
            sp::ADMIN => {
                self.admin = preconfig_admin::preconfig();
                self.locking = preconfig_locking::preconfig();
                Ok(vec![sp::ADMIN, sp::LOCKING])
            }
            sp::LOCKING => {
                if let Some(locking_sp) = self.admin.sp.get_mut(&sp::LOCKING) {
                    locking_sp.life_cycle_state = Some(LifeCycleState::Manufactured);
                }
                self.locking = preconfig_locking::preconfig();
                Ok(vec![sp::LOCKING])
            }
            _ => Err(MethodStatus::InvalidParameter),
        }
    }
}

impl Default for EnterpriseTper {
    fn default() -> Self {
        Self { admin: preconfig_admin::preconfig(), locking: preconfig_locking::preconfig() }
    }
}
