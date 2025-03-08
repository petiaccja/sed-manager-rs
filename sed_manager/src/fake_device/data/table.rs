use core::ops::{Deref, DerefMut};
use std::collections::BTreeMap;

use crate::{
    messaging::uid::{TableUID, UID},
    spec::{
        column_types::{AuthorityRef, CPINRef, KAES256Ref, LockingRangeRef, SPRef},
        objects::{Authority, LockingRange, CPIN, KAES256, SP},
        table_id,
    },
};

use super::object::GenericObject;

pub type AuthorityTable = Table<Authority, AuthorityRef, { table_id::AUTHORITY.as_u64() }>;
pub type CPINTable = Table<CPIN, CPINRef, { table_id::C_PIN.as_u64() }>;
pub type KAES256Table = Table<KAES256, KAES256Ref, { table_id::K_AES_256.as_u64() }>;
pub type LockingTable = Table<LockingRange, LockingRangeRef, { table_id::LOCKING.as_u64() }>;
pub type SPTable = Table<SP, SPRef, { table_id::SP.as_u64() }>;

pub trait GenericTable {
    fn uid(&self) -> TableUID;
    fn get_object(&self, uid: UID) -> Option<&dyn GenericObject>;
    fn get_object_mut(&mut self, uid: UID) -> Option<&mut dyn GenericObject>;
    fn next_from(&self, uid: Option<UID>) -> Option<UID>;
}

pub struct Table<Object, ObjectRef, const THIS_TABLE: u64>(BTreeMap<ObjectRef, Object>)
where
    Object: GenericObject,
    ObjectRef: TryFrom<UID> + Into<UID> + Copy;

impl<Object, ObjectRef, const THIS_TABLE: u64> Table<Object, ObjectRef, THIS_TABLE>
where
    Object: GenericObject,
    ObjectRef: TryFrom<UID> + Into<UID> + Ord + Copy,
{
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }
}

impl<Object, ObjectRef, const THIS_TABLE: u64> Deref for Table<Object, ObjectRef, THIS_TABLE>
where
    Object: GenericObject,
    ObjectRef: TryFrom<UID> + Into<UID> + Ord + Copy,
{
    type Target = BTreeMap<ObjectRef, Object>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<Object, ObjectRef, const THIS_TABLE: u64> DerefMut for Table<Object, ObjectRef, THIS_TABLE>
where
    Object: GenericObject,
    ObjectRef: TryFrom<UID> + Into<UID> + Ord + Copy,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<Object, ObjectRef, const THIS_TABLE: u64> GenericTable for Table<Object, ObjectRef, THIS_TABLE>
where
    Object: GenericObject,
    ObjectRef: TryFrom<UID> + Into<UID> + Ord + Copy,
{
    fn uid(&self) -> TableUID {
        TableUID::new(THIS_TABLE)
    }

    fn get_object(&self, uid: UID) -> Option<&dyn GenericObject> {
        if let Ok(uid) = uid.try_into() {
            self.0.get(&uid).map(|object| object as &dyn GenericObject)
        } else {
            None
        }
    }

    fn get_object_mut(&mut self, uid: UID) -> Option<&mut dyn GenericObject> {
        if let Ok(uid) = uid.try_into() {
            self.0.get_mut(&uid).map(|object| object as &mut dyn GenericObject)
        } else {
            None
        }
    }

    fn next_from(&self, uid: Option<UID>) -> Option<UID> {
        let uid = match uid {
            Some(uid) => Some(ObjectRef::try_from(uid).ok()?),
            None => None,
        };
        if let Some(uid) = uid {
            let mut iter = self.0.range(uid..);
            if iter.next().is_none() {
                None
            } else {
                iter.next().map(|(k, _v)| (*k).into())
            }
        } else {
            let mut iter = self.0.range(..);
            iter.next().map(|(k, _v)| (*k).into())
        }
    }
}
