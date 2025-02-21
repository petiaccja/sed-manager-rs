use as_array::AsArray;

use crate::messaging::uid::UID;
use crate::messaging::value::Value;
use crate::spec::basic_types::ByteTableReference;
use crate::spec::column_types::{CPINRef, Name, Password};

use super::super::field::Field;
use super::super::object::Object;

#[derive(AsArray)]
#[as_array_traits(Field)]
pub struct CPIN {
    pub uid: CPINRef,
    pub name: Option<Name>,
    pub common_name: Option<Name>,
    pub pin: Option<Password>,
    pub char_set: Option<ByteTableReference>,
    pub try_limit: Option<u32>,
    pub tries: Option<u32>,
    pub persistence: Option<bool>,
}

impl CPIN {
    pub fn new(uid: CPINRef) -> Self {
        Self {
            uid,
            name: None,
            common_name: None,
            pin: None,
            char_set: None,
            try_limit: None,
            tries: None,
            persistence: None,
        }
    }
}

impl Object for CPIN {
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
