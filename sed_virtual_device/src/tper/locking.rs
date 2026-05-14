use std::any::Any;

use sed_spec::objects::{
    AccessControl, Ace, Authority, CPin, KAes256, LockingRange, MbrControl, SecurityProviderRef, TableDesc,
};

use crate::tper::security_provider::{SecurityProvider, Table};

#[derive(Debug)]
pub struct Locking {
    pub uid: SecurityProviderRef,
    pub access_control: Table<AccessControl>,
    pub ace: Table<Ace>,
    pub authority: Table<Authority>,
    pub c_pin: Table<CPin>,
    pub k_aes_256: Table<KAes256>,
    pub locking: Table<LockingRange>,
    pub mbr_control: Table<MbrControl>,
    pub table: Table<TableDesc>,
    pub mbr: Vec<u8>,
    pub data_store: Vec<Vec<u8>>,
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

    fn k_aes_256(&self) -> Option<&Table<KAes256>> {
        Some(&self.k_aes_256)
    }

    fn k_aes_256_mut(&mut self) -> Option<&mut Table<KAes256>> {
        Some(&mut self.k_aes_256)
    }

    fn locking(&self) -> Option<&Table<LockingRange>> {
        Some(&self.locking)
    }

    fn locking_mut(&mut self) -> Option<&mut Table<LockingRange>> {
        Some(&mut self.locking)
    }

    fn mbr_control(&self) -> Option<&Table<MbrControl>> {
        Some(&self.mbr_control)
    }

    fn mbr_control_mut(&mut self) -> Option<&mut Table<MbrControl>> {
        Some(&mut self.mbr_control)
    }

    fn mbr(&self) -> Option<&Vec<u8>> {
        Some(&self.mbr)
    }

    fn mbr_mut(&mut self) -> Option<&mut Vec<u8>> {
        Some(&mut self.mbr)
    }

    fn data_store(&self, index: usize) -> Option<&Vec<u8>> {
        self.data_store.get(index)
    }

    fn data_store_mut(&mut self, index: usize) -> Option<&mut Vec<u8>> {
        self.data_store.get_mut(index)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
