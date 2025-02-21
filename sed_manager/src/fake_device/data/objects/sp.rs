use as_array::AsArray;

use crate::messaging::uid::UID;
use crate::messaging::value::Value;
use crate::spec::column_types::{AuthorityRef, Date, LifeCycleState, MaxBytes32, Name, SPRef};

use super::super::field::Field;
use super::super::object::Object;

#[derive(AsArray)]
#[as_array_traits(Field)]
pub struct SP {
    pub uid: SPRef,
    pub name: Name,
    pub org: Option<AuthorityRef>,
    pub effective_auth: Option<MaxBytes32>,
    pub date_of_issue: Option<Date>,
    pub bytes: Option<u64>,
    pub life_cycle_state: LifeCycleState,
    pub frozen: bool,
}

impl SP {
    pub fn new(uid: SPRef, name: String, life_cycle_state: LifeCycleState) -> Self {
        Self {
            uid,
            name: name.into(),
            org: None,
            effective_auth: None,
            date_of_issue: None,
            bytes: None,
            life_cycle_state,
            frozen: false,
        }
    }
}

impl Object for SP {
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
