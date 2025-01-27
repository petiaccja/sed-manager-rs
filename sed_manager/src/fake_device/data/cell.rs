use crate::messaging::{types::Type, value::Value};

pub trait Cell {
    fn get(&self) -> Value;
    fn try_set(&mut self, value: Value) -> Result<(), Value>;
    fn is_empty(&self) -> bool;
}

pub struct CellData<T>(Option<T>)
where
    T: Type,
    T: TryFrom<Value>,
    Value: From<T>;

impl<T> CellData<T>
where
    T: Type,
    T: TryFrom<Value>,
    Value: From<T>,
{
    pub fn get(&self) -> Option<&T> {
        self.0.as_ref()
    }

    pub fn get_mut(&mut self) -> Option<&mut T> {
        self.0.as_mut()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_some()
    }
}

impl<T> Cell for CellData<T>
where
    T: Type,
    T: TryFrom<Value, Error = Value>,
    Value: From<T>,
    T: Clone,
{
    fn get(&self) -> Value {
        self.get().map(|data| Value::from(data.clone())).unwrap_or(Value::empty())
    }

    fn try_set(&mut self, value: Value) -> Result<(), Value> {
        self.get_mut().replace(&mut (T::try_from(value)?));
        Ok(())
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}
