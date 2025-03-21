//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::messaging::value::Value;

pub trait Cell {
    fn get(&self) -> Value;
    fn try_replace(&mut self, value: Value) -> Result<Value, Value>;
}

impl<T> Cell for T
where
    T: TryFrom<Value, Error = Value> + Into<Value>,
    Self: Clone,
{
    fn get(&self) -> Value {
        self.clone().into()
    }

    fn try_replace(&mut self, value: Value) -> Result<Value, Value> {
        let new = Self::try_from(value)?;
        Ok(core::mem::replace(self, new).into())
    }
}
