use super::value::{Bytes, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UID {
    value: u64,
}

impl From<UID> for Value {
    fn from(value: UID) -> Self {
        Value::from(Bytes::from(value.value.to_be_bytes()))
    }
}

impl TryFrom<Value> for UID {
    type Error = Value;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match <&Bytes>::try_from(&value) {
            Ok(bytes) => match <[u8; 8]>::try_from(bytes.as_slice()) {
                Ok(fixed_bytes) => Ok(UID { value: u64::from_be_bytes(fixed_bytes) }),
                Err(_) => Err(value),
            },
            Err(_) => Err(value),
        }
    }
}

impl From<u64> for UID {
    fn from(value: u64) -> Self {
        Self { value: value }
    }
}

impl From<u32> for UID {
    fn from(value: u32) -> Self {
        Self { value: value as u64 }
    }
}

impl From<u16> for UID {
    fn from(value: u16) -> Self {
        Self { value: value as u64 }
    }
}

impl From<u8> for UID {
    fn from(value: u8) -> Self {
        Self { value: value as u64 }
    }
}
