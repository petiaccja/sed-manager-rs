use crate::fake_device::data::objects::{AuthorityTable, CPinTable};
use crate::fake_device::data::table::BasicTable;
use crate::messaging::types::SPRef;
use crate::messaging::uid::UID;
use crate::specification::{sp, table};

use super::super::SecurityProvider;

pub struct LockingSP {
    authorities: AuthorityTable,
    c_pin: CPinTable,
}

impl LockingSP {
    pub fn new() -> Self {
        Self { authorities: AuthorityTable::new(), c_pin: CPinTable::new() }
    }
}

impl SecurityProvider for LockingSP {
    fn uid(&self) -> SPRef {
        sp::ADMIN.into()
    }

    fn get_authority_table(&self) -> Option<&AuthorityTable> {
        Some(&self.authorities)
    }

    fn get_c_pin_table(&self) -> Option<&CPinTable> {
        Some(&self.c_pin)
    }

    fn get_table(&self, uid: UID) -> Option<&dyn BasicTable> {
        match uid {
            table::AUTHORITY => Some(&self.authorities as &dyn BasicTable),
            table::C_PIN => Some(&self.c_pin as &dyn BasicTable),
            _ => None,
        }
    }

    fn get_table_mut(&mut self, uid: UID) -> Option<&mut dyn BasicTable> {
        match uid {
            table::AUTHORITY => Some(&mut self.authorities as &mut dyn BasicTable),
            table::C_PIN => Some(&mut self.c_pin as &mut dyn BasicTable),
            _ => None,
        }
    }
}
