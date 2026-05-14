use std::{any::Any, collections::BTreeMap};

use sed_packet::Object;
use sed_spec::objects::{
    AccessControl, Ace, Authority, CPin, KAes256, LockingRange, MbrControl, SecurityProvider as SecurityProviderObj,
    TableDesc,
};

pub type Table<T> = BTreeMap<<T as Object>::Ref, T>;

pub trait SecurityProvider {
    // Tables all SPs have.
    fn access_control(&self) -> &Table<AccessControl>;
    #[allow(unused)]
    fn access_control_mut(&mut self) -> &mut Table<AccessControl>;
    fn ace(&self) -> &Table<Ace>;
    fn ace_mut(&mut self) -> &mut Table<Ace>;
    fn authority(&self) -> &Table<Authority>;
    fn authority_mut(&mut self) -> &mut Table<Authority>;
    fn c_pin(&self) -> &Table<CPin>;
    fn c_pin_mut(&mut self) -> &mut Table<CPin>;
    fn table(&self) -> &Table<TableDesc>;
    fn table_mut(&mut self) -> &mut Table<TableDesc>;

    // Tables some SPs have.
    fn k_aes_256(&self) -> Option<&Table<KAes256>> {
        None
    }
    fn k_aes_256_mut(&mut self) -> Option<&mut Table<KAes256>> {
        None
    }
    fn locking(&self) -> Option<&Table<LockingRange>> {
        None
    }
    fn locking_mut(&mut self) -> Option<&mut Table<LockingRange>> {
        None
    }
    fn mbr_control(&self) -> Option<&Table<MbrControl>> {
        None
    }
    fn mbr_control_mut(&mut self) -> Option<&mut Table<MbrControl>> {
        None
    }
    fn sp(&self) -> Option<&Table<SecurityProviderObj>> {
        None
    }
    fn sp_mut(&mut self) -> Option<&mut Table<SecurityProviderObj>> {
        None
    }
    fn mbr(&self) -> Option<&Vec<u8>> {
        None
    }
    fn mbr_mut(&mut self) -> Option<&mut Vec<u8>> {
        None
    }
    fn data_store(&self, index: usize) -> Option<&Vec<u8>> {
        let _ = index;
        None
    }
    fn data_store_mut(&mut self, index: usize) -> Option<&mut Vec<u8>> {
        let _ = index;
        None
    }

    // Type-erased SPs.
    #[allow(unused)]
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
