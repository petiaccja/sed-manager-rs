//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::messaging::uid::UID;
use crate::messaging::value::Value;
use crate::spec::objects::{Authority, LockingRange, MBRControl, TableDesc, ACE, CPIN, KAES256, SP};

pub trait GenericObject {
    fn uid(&self) -> UID;
    fn len(&self) -> usize;
    fn get(&self, column: usize) -> Value;
    fn try_replace(&mut self, column: usize, value: Value) -> Result<Value, Value>;
}

macro_rules! impl_generic_object {
    ($type:ty) => {
        impl GenericObject for $type {
            fn uid(&self) -> UID {
                UID::try_from(self.get(0)).unwrap()
            }

            fn len(&self) -> usize {
                self.as_array().len()
            }

            fn get(&self, column: usize) -> Value {
                self.as_array()[column].get()
            }

            fn try_replace(&mut self, column: usize, value: Value) -> Result<Value, Value> {
                self.as_array_mut()[column].try_replace(value)
            }
        }
    };
}

impl_generic_object!(Authority);
impl_generic_object!(ACE);
impl_generic_object!(CPIN);
impl_generic_object!(KAES256);
impl_generic_object!(LockingRange);
impl_generic_object!(SP);
impl_generic_object!(TableDesc);
impl_generic_object!(MBRControl);
