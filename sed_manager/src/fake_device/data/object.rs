use crate::messaging::uid::UID;
use crate::messaging::value::Value;

pub trait GenericObject {
    fn uid(&self) -> UID;
    fn len(&self) -> usize;
    fn is_column_empty(&self, column: usize) -> bool;
    fn get_column(&self, column: usize) -> Value;
    fn try_set_column(&mut self, column: usize, value: Value) -> Result<(), Value>;
}
