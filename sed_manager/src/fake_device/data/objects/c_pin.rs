use as_array::AsArray;

use crate::fake_device::data::{Field, Object};
use crate::messaging::uid::UID;
use crate::messaging::value::Value;
use crate::specification::basic_types::ByteTableReference;
use crate::specification::column_types::{CPinRef, Name, Password};

#[derive(AsArray, Default)]
#[as_array_traits(Field)]
pub struct CPin {
    pub uid: CPinRef,
    pub name: Option<Name>,
    pub common_name: Option<Name>,
    pub pin: Option<Password>,
    pub char_set: Option<ByteTableReference>,
    pub try_limit: Option<u32>,
    pub tries: Option<u32>,
    pub persistence: Option<bool>,
}

impl CPin {
    pub fn new(uid: CPinRef) -> Self {
        Self { uid, ..Default::default() }
    }
}

impl Object for CPin {
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
