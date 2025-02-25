use core::ops::Deref;

use crate::messaging::value::{Bytes, Value};

#[derive(PartialEq, Eq, Clone, Debug, Default)]
pub struct MaxBytes<const LIMIT: usize>(pub Bytes);

impl<const LIMIT: usize> Deref for MaxBytes<LIMIT> {
    type Target = Bytes;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const LIMIT: usize> From<MaxBytes<LIMIT>> for Bytes {
    fn from(value: MaxBytes<LIMIT>) -> Self {
        value.0
    }
}

impl<const LIMIT: usize> From<Bytes> for MaxBytes<LIMIT> {
    fn from(value: Bytes) -> Self {
        Self(value)
    }
}

impl<const LIMIT: usize> From<MaxBytes<LIMIT>> for Value {
    fn from(value: MaxBytes<LIMIT>) -> Self {
        Value::from(value.0)
    }
}

impl<const LIMIT: usize> From<&MaxBytes<LIMIT>> for Value {
    fn from(value: &MaxBytes<LIMIT>) -> Self {
        Value::from(value.0.clone())
    }
}

impl<const LIMIT: usize> TryFrom<Value> for MaxBytes<LIMIT> {
    type Error = Value;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        Ok(Self(Bytes::try_from(value)?))
    }
}

impl<const LIMIT: usize> TryFrom<MaxBytes<LIMIT>> for String {
    type Error = MaxBytes<LIMIT>;
    fn try_from(value: MaxBytes<LIMIT>) -> Result<Self, Self::Error> {
        match String::from_utf8(value.0) {
            Ok(s) => Ok(s),
            Err(err) => Err(err.into_bytes().into()),
        }
    }
}

impl<const LIMIT: usize> From<String> for MaxBytes<LIMIT> {
    fn from(value: String) -> Self {
        Self(value.into_bytes())
    }
}

impl<const LIMIT: usize> From<&str> for MaxBytes<LIMIT> {
    fn from(value: &str) -> Self {
        Self(value.as_bytes().into())
    }
}
