use as_array::AsArray;

use crate::messaging::uid::UID;
use crate::messaging::value::Value;
use crate::spec::column_types::{KAES256Ref, Key256, Name, SymmetricModeMedia};

use super::super::field::Field;
use super::super::object::GenericObject;

#[derive(AsArray)]
#[as_array_traits(Field)]
pub struct KAES256 {
    pub uid: KAES256Ref,
    pub name: Option<Name>,
    pub common_name: Option<Name>,
    pub key: Option<Key256>,
    pub mode: Option<SymmetricModeMedia>,
}

impl KAES256 {
    pub fn new(uid: KAES256Ref) -> Self {
        Self { uid, name: None, common_name: None, key: None, mode: None }
    }
}

impl GenericObject for KAES256 {
    fn uid(&self) -> UID {
        self.uid.into()
    }

    fn len(&self) -> usize {
        self.as_array().len()
    }

    fn is_column_empty(&self, column: usize) -> bool {
        self.as_array()[column].is_empty()
    }

    fn get_column(&self, column: usize) -> crate::messaging::value::Value {
        self.as_array()[column].to_value()
    }

    fn try_set_column(&mut self, column: usize, value: Value) -> Result<(), Value> {
        self.as_array_mut()[column].try_replace_with_value(value)
    }
}
