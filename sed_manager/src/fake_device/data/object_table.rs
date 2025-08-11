//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::any::Any;
use core::ops::{Deref, DerefMut};
use std::collections::BTreeMap;

use crate::messaging::uid::{TableUID, UID};
use crate::spec::column_types::{
    ACERef, AuthorityRef, CPINRef, KAES256Ref, LockingRangeRef, MBRControlRef, SPRef, TableDescRef,
};
use crate::spec::objects::{Authority, LockingRange, MBRControl, TableDesc, ACE, CPIN, KAES256, SP};
use crate::spec::table_id;

use super::object::GenericObject;

pub type AuthorityTable = ObjectTable<Authority, AuthorityRef, { table_id::AUTHORITY.as_u64() }>;
pub type ACETable = ObjectTable<ACE, ACERef, { table_id::ACE.as_u64() }>;
pub type TableTable = ObjectTable<TableDesc, TableDescRef, { table_id::TABLE.as_u64() }>;
pub type MBRControlTable = ObjectTable<MBRControl, MBRControlRef, { table_id::MBR_CONTROL.as_u64() }>;
pub type CPINTable = ObjectTable<CPIN, CPINRef, { table_id::C_PIN.as_u64() }>;
pub type KAES256Table = ObjectTable<KAES256, KAES256Ref, { table_id::K_AES_256.as_u64() }>;
pub type LockingTable = ObjectTable<LockingRange, LockingRangeRef, { table_id::LOCKING.as_u64() }>;
pub type SPTable = ObjectTable<SP, SPRef, { table_id::SP.as_u64() }>;

pub trait GenericTable: Send + Sync {
    fn uid(&self) -> TableUID;
    fn get_object(&self, uid: UID) -> Option<&dyn GenericObject>;
    fn get_object_mut(&mut self, uid: UID) -> Option<&mut dyn GenericObject>;
    fn next_from(&self, uid: Option<UID>) -> Option<UID>;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub struct ObjectTable<Object, ObjectRef, const THIS_TABLE: u64>(BTreeMap<ObjectRef, Object>)
where
    Object: GenericObject,
    ObjectRef: TryFrom<UID> + Into<UID> + Copy;

impl<Object, ObjectRef, const THIS_TABLE: u64> ObjectTable<Object, ObjectRef, THIS_TABLE>
where
    Object: GenericObject,
    ObjectRef: TryFrom<UID> + Into<UID> + Ord + Copy,
{
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }
}

impl<Object, ObjectRef, const THIS_TABLE: u64> Deref for ObjectTable<Object, ObjectRef, THIS_TABLE>
where
    Object: GenericObject,
    ObjectRef: TryFrom<UID> + Into<UID> + Ord + Copy,
{
    type Target = BTreeMap<ObjectRef, Object>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<Object, ObjectRef, const THIS_TABLE: u64> DerefMut for ObjectTable<Object, ObjectRef, THIS_TABLE>
where
    Object: GenericObject,
    ObjectRef: TryFrom<UID> + Into<UID> + Ord + Copy,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<Object, ObjectRef, const THIS_TABLE: u64> FromIterator<Object> for ObjectTable<Object, ObjectRef, THIS_TABLE>
where
    Object: GenericObject,
    ObjectRef: TryFrom<UID> + Into<UID> + Ord + Copy,
{
    fn from_iter<T: IntoIterator<Item = Object>>(iter: T) -> Self {
        Self(
            iter.into_iter()
                .map(|object| (object.uid().try_into().unwrap_or_else(|_| panic!()), object))
                .collect(),
        )
    }
}

impl<Object, ObjectRef, const THIS_TABLE: u64> GenericTable for ObjectTable<Object, ObjectRef, THIS_TABLE>
where
    Object: GenericObject + Send + Sync + 'static,
    ObjectRef: TryFrom<UID> + Into<UID> + Ord + Copy + Send + Sync + 'static,
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

    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }
}
