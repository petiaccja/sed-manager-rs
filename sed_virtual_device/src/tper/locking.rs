use sed_spec::objects::{AccessControl, Ace, Authority, CPin, KAes256, LockingRange, MbrControl, TableDesc};

use crate::tper::security_provider::{SecurityProvider, Table};

pub struct Locking {
    pub access_control: Table<AccessControl>,
    pub ace: Table<Ace>,
    pub authority: Table<Authority>,
    pub c_pin: Table<CPin>,
    pub k_aes_256: Table<KAes256>,
    pub locking: Table<LockingRange>,
    pub mbr_control: Table<MbrControl>,
    pub table: Table<TableDesc>,
    pub mbr: Vec<u8>,
}

impl SecurityProvider for Locking {
    fn access_control(&self) -> &Table<AccessControl> {
        &self.access_control
    }

    fn access_control_mut(&mut self) -> &mut Table<AccessControl> {
        &mut self.access_control
    }

    fn ace(&self) -> &Table<Ace> {
        &self.ace
    }

    fn ace_mut(&mut self) -> &mut Table<Ace> {
        &mut self.ace
    }

    fn authority(&self) -> &Table<Authority> {
        &self.authority
    }

    fn authority_mut(&mut self) -> &mut Table<Authority> {
        &mut self.authority
    }

    fn c_pin(&self) -> &Table<CPin> {
        &self.c_pin
    }

    fn c_pin_mut(&mut self) -> &mut Table<CPin> {
        &mut self.c_pin
    }

    fn table(&self) -> &Table<TableDesc> {
        &self.table
    }

    fn table_mut(&mut self) -> &mut Table<TableDesc> {
        &mut self.table
    }
}
