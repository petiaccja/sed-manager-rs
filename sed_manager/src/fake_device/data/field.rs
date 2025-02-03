use crate::messaging::value::Value;

pub trait Field {
    fn to_value(&self) -> Value;
    fn try_replace_with_value(&mut self, value: Value) -> Result<(), Value>;
    fn is_empty(&self) -> bool;
}

impl<T> Field for T
where
    T: TryFrom<Value, Error = Value> + Into<Value> + Clone,
{
    fn to_value(&self) -> Value {
        self.clone().into()
    }

    fn try_replace_with_value(&mut self, value: Value) -> Result<(), Value> {
        let _ = std::mem::replace(self, Self::try_from(value)?);
        Ok(())
    }

    fn is_empty(&self) -> bool {
        false
    }
}

impl<T> Field for Option<T>
where
    T: Field + TryFrom<Value, Error = Value>,
{
    fn to_value(&self) -> Value {
        if let Some(data) = self.as_ref() {
            data.to_value()
        } else {
            Value::empty()
        }
    }

    fn try_replace_with_value(&mut self, value: Value) -> Result<(), Value> {
        self.replace(T::try_from(value)?);
        Ok(())
    }

    fn is_empty(&self) -> bool {
        self.is_none()
    }
}
