//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::messaging::value::Value;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct NamedValue<NameTy, ValueTy> {
    pub name: NameTy,
    pub value: ValueTy,
}

impl<NameTy: Into<Value>, ValueTy: Into<Value>> From<NamedValue<NameTy, ValueTy>> for Value {
    fn from(value: NamedValue<NameTy, ValueTy>) -> Self {
        Value::from(crate::messaging::value::Named { name: value.name.into(), value: value.value.into() })
    }
}

impl<NameTy: TryFrom<Value>, ValueTy: TryFrom<Value>> TryFrom<Value> for NamedValue<NameTy, ValueTy> {
    type Error = Value;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let named = crate::messaging::value::Named::try_from(value)?;
        if let (Ok(name), Ok(value)) = (NameTy::try_from(named.name.clone()), ValueTy::try_from(named.value.clone())) {
            Ok(NamedValue { name, value })
        } else {
            Err(Value::from(named))
        }
    }
}
