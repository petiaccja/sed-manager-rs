use std::collections::HashMap;

use crate::spec::column_types::SPRef;
use crate::spec::opal;

use super::security_providers::{AdminSP, LockingSP};
use super::SecurityProvider;

type BoxedSSC = Box<dyn SecurityProvider + Send + Sync>;

pub struct SSC {
    security_providers: HashMap<SPRef, BoxedSSC>,
}

impl SSC {
    pub fn new() -> Self {
        let mut security_providers = HashMap::new();
        security_providers.insert(opal::admin::sp::ADMIN, BoxedSSC::from(Box::new(AdminSP::new())));
        security_providers.insert(opal::admin::sp::LOCKING, BoxedSSC::from(Box::new(LockingSP::new())));
        Self { security_providers }
    }

    pub fn has_security_provider(&self, sp: SPRef) -> bool {
        self.get_security_provider(sp).is_some()
    }

    pub fn get_security_provider(&self, sp: SPRef) -> Option<&dyn SecurityProvider> {
        match self.security_providers.get(&sp) {
            Some(boxed) => Some(boxed.as_ref()),
            None => None,
        }
    }

    pub fn get_security_provider_mut(&mut self, sp: SPRef) -> Option<&mut dyn SecurityProvider> {
        match self.security_providers.get_mut(&sp) {
            Some(boxed) => Some(boxed.as_mut()),
            None => None,
        }
    }
}
