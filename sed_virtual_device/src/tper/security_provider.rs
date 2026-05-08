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
    fn k_aes_256(&self) -> Option<&Table<KAes256>>;
    fn locking(&self) -> Option<&Table<LockingRange>>;
    fn mbr_control(&self) -> Option<&Table<MbrControl>>;
    fn sp(&self) -> Option<&Table<SecurityProviderObj>>;

    // Type-erased SPs.
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
