use std::collections::HashMap;

use crate::specification::column_types::SPRef;
use crate::specification::opal;

use super::security_providers::{AdminSP, LockingSP};
use super::SecurityProvider;

type BoxedSSC = Box<dyn SecurityProvider + Send + Sync>;

pub struct SSC {
    security_providers: HashMap<SPRef, BoxedSSC>,
}

impl SSC {
    pub fn new() -> Self {
        let mut security_providers = HashMap::new();
        security_providers.insert(opal::admin::sp::ADMIN.into(), BoxedSSC::from(Box::new(AdminSP::new())));
        security_providers.insert(opal::admin::sp::LOCKING.into(), BoxedSSC::from(Box::new(LockingSP::new())));
        Self { security_providers }
    }

    pub fn get_sp(&self, sp: SPRef) -> Option<&dyn SecurityProvider> {
        match self.security_providers.get(&sp) {
            Some(boxed) => Some(boxed.as_ref()),
            None => None,
        }
    }

    pub fn get_sp_mut(&mut self, sp: SPRef) -> Option<&mut dyn SecurityProvider> {
        match self.security_providers.get_mut(&sp) {
            Some(boxed) => Some(boxed.as_mut()),
            None => None,
        }
    }
}
