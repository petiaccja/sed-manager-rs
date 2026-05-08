use std::any::Any;

use sed_spec::objects::{
    AccessControl, Ace, Authority, CPin, KAes256, LockingRange, MbrControl, SecurityProvider as SecurityProviderObj,
    SecurityProviderRef, TableDesc,
};

use crate::tper::security_provider::{SecurityProvider, Table};

#[derive(Debug)]
pub struct Admin {
    pub uid: SecurityProviderRef,
    pub access_control: Table<AccessControl>,
    pub ace: Table<Ace>,
    pub authority: Table<Authority>,
    pub c_pin: Table<CPin>,
    pub sp: Table<SecurityProviderObj>,
    pub table: Table<TableDesc>,
}

impl SecurityProvider for Admin {
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

    fn k_aes_256(&self) -> Option<&Table<KAes256>> {
        None
    }

    fn locking(&self) -> Option<&Table<LockingRange>> {
        None
    }

    fn mbr_control(&self) -> Option<&Table<MbrControl>> {
        None
    }

    fn sp(&self) -> Option<&Table<SecurityProviderObj>> {
        Some(&self.sp)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
