use crate::{messaging::uid::UID, specification::sp};

use super::security_provider::SecurityProvider;

pub struct Controller {
    admin_sp: SecurityProvider,
    locking_sp: SecurityProvider,
}

impl Controller {
    pub fn new() -> Self {
        Self { admin_sp: SecurityProvider::new(sp::ADMIN), locking_sp: SecurityProvider::new(sp::LOCKING) }
    }

    pub fn get_sp(&self, sp: UID) -> Option<&SecurityProvider> {
        if sp == self.admin_sp.uid() {
            Some(&self.admin_sp)
        } else if sp == self.locking_sp.uid() {
            Some(&self.locking_sp)
        } else {
            None
        }
    }

    pub fn get_sp_mut(&mut self, sp: UID) -> Option<&mut SecurityProvider> {
        if sp == self.admin_sp.uid() {
            Some(&mut self.admin_sp)
        } else if sp == self.locking_sp.uid() {
            Some(&mut self.locking_sp)
        } else {
            None
        }
    }
}
