use std::vec::Vec;

pub type Bytes = Vec<u8>;
pub type List = Vec<Value>;

#[repr(u8)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Copy, Clone)]
pub enum Command {
    Call = 0xF8,
    EndOfData = 0xF9,
    EndOfSession = 0xFA,
    StartTransaction = 0xFB,
    EndTransaction = 0xFC,
    Empty = 0xFF,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Named {
    pub name: Value,
    pub value: Value,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Storage {
    Empty,
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Uint8(u8),
    Uint16(u16),
    Uint32(u32),
    Uint64(u64),
    Command(Command),
    Named(Box<Named>),
    Bytes(Bytes),
    List(List),
}

#[derive(Debug)]
pub enum ValueConversionError {
    Fail,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Value {
    storage: Storage,
}

impl Value {
    pub fn empty() -> Self {
        Self { storage: Storage::Empty }
    }
    pub fn storage<'value>(&'value self) -> &'value Storage {
        &self.storage
    }
    pub fn is_empty(&self) -> bool {
        match &self.storage {
            Storage::Empty => true,
            _ => false,
        }
    }
    pub fn is_empty_command(&self) -> bool {
        match &self.storage {
            Storage::Command(Command::Empty) => true,
            _ => false,
        }
    }
}

macro_rules! impl_value_from {
    ($storage_ty:ty, $enum_variant:expr) => {
        impl From<$storage_ty> for Value {
            fn from(value: $storage_ty) -> Self {
                Self { storage: $enum_variant(value) }
            }
        }
    };
}

impl_value_from!(i8, Storage::Int8);
impl_value_from!(i16, Storage::Int16);
impl_value_from!(i32, Storage::Int32);
impl_value_from!(i64, Storage::Int64);
impl_value_from!(u8, Storage::Uint8);
impl_value_from!(u16, Storage::Uint16);
impl_value_from!(u32, Storage::Uint32);
impl_value_from!(u64, Storage::Uint64);
impl_value_from!(Command, Storage::Command);
impl_value_from!(Bytes, Storage::Bytes);
impl_value_from!(List, Storage::List);

impl From<Named> for Value {
    fn from(value: Named) -> Self {
        Self { storage: Storage::Named(Box::new(value)) }
    }
}

macro_rules! impl_value_try_into {
    { $storage_ty:ty, $enum_variant:pat, $value_expr:ident} => {
        impl TryFrom<Value> for $storage_ty {
            type Error = ValueConversionError;
            fn try_from(value: Value) -> Result<Self, Self::Error> {
                match value.storage {
                    $enum_variant => Ok($value_expr),
                    _ => Err(ValueConversionError::Fail),
                }
            }
        }

        impl<'value> TryFrom<&'value Value> for $storage_ty {
            type Error = ValueConversionError;
            fn try_from(value: &'value Value) -> Result<Self, Self::Error> {
                match value.storage {
                    $enum_variant => Ok($value_expr),
                    _ => Err(ValueConversionError::Fail),
                }
            }
        }
    };
}

impl_value_try_into!(i8, Storage::Int8(value), value);
impl_value_try_into!(i16, Storage::Int16(value), value);
impl_value_try_into!(i32, Storage::Int32(value), value);
impl_value_try_into!(i64, Storage::Int64(value), value);
impl_value_try_into!(u8, Storage::Uint8(value), value);
impl_value_try_into!(u16, Storage::Uint16(value), value);
impl_value_try_into!(u32, Storage::Uint32(value), value);
impl_value_try_into!(u64, Storage::Uint64(value), value);
impl_value_try_into!(Command, Storage::Command(value), value);

impl TryFrom<Value> for Named {
    type Error = ValueConversionError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value.storage {
            Storage::Named(value) => Ok(*value),
            _ => Err(ValueConversionError::Fail),
        }
    }
}

impl TryFrom<Value> for Bytes {
    type Error = ValueConversionError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value.storage {
            Storage::Bytes(value) => Ok(value),
            _ => Err(ValueConversionError::Fail),
        }
    }
}

impl TryFrom<Value> for List {
    type Error = ValueConversionError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value.storage {
            Storage::List(value) => Ok(value),
            _ => Err(ValueConversionError::Fail),
        }
    }
}

impl<'value> TryFrom<&'value Value> for &'value Named {
    type Error = ValueConversionError;
    fn try_from(value: &'value Value) -> Result<Self, Self::Error> {
        match value.storage {
            Storage::Named(ref value) => Ok(&*value),
            _ => Err(ValueConversionError::Fail),
        }
    }
}

impl<'value> TryFrom<&'value Value> for &'value Bytes {
    type Error = ValueConversionError;
    fn try_from(value: &'value Value) -> Result<Self, Self::Error> {
        match value.storage {
            Storage::Bytes(ref value) => Ok(value),
            _ => Err(ValueConversionError::Fail),
        }
    }
}

impl<'value> TryFrom<&'value Value> for &'value List {
    type Error = ValueConversionError;
    fn try_from(value: &'value Value) -> Result<Self, Self::Error> {
        match value.storage {
            Storage::List(ref value) => Ok(value),
            _ => Err(ValueConversionError::Fail),
        }
    }
}

impl TryFrom<&Value> for Named {
    type Error = ValueConversionError;
    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value.storage {
            Storage::Named(ref value) => Ok(value.as_ref().clone()),
            _ => Err(ValueConversionError::Fail),
        }
    }
}

impl TryFrom<&Value> for Bytes {
    type Error = ValueConversionError;
    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value.storage {
            Storage::Bytes(ref value) => Ok(value.clone()),
            _ => Err(ValueConversionError::Fail),
        }
    }
}

impl TryFrom<&Value> for List {
    type Error = ValueConversionError;
    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value.storage {
            Storage::List(ref value) => Ok(value.clone()),
            _ => Err(ValueConversionError::Fail),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_from_i8() {
        let input = 1i8;
        let value = Value::from(input);
        let content: i8 = value.try_into().unwrap();
        assert_eq!(content, input);
    }

    #[test]
    fn value_from_i16() {
        let input = 1i16;
        let value = Value::from(input);
        let content: i16 = value.try_into().unwrap();
        assert_eq!(content, input);
    }

    #[test]
    fn value_from_i32() {
        let input = 1i32;
        let value = Value::from(input);
        let content: i32 = value.try_into().unwrap();
        assert_eq!(content, input);
    }

    #[test]
    fn value_from_i64() {
        let input = 1i64;
        let value = Value::from(input);
        let content: i64 = value.try_into().unwrap();
        assert_eq!(content, input);
    }

    #[test]
    fn value_from_u8() {
        let input = 1u8;
        let value = Value::from(input);
        let content: u8 = value.try_into().unwrap();
        assert_eq!(content, input);
    }

    #[test]
    fn value_from_u16() {
        let input = 1u16;
        let value = Value::from(input);
        let content: u16 = value.try_into().unwrap();
        assert_eq!(content, input);
    }

    #[test]
    fn value_from_u32() {
        let input = 1u32;
        let value = Value::from(input);
        let content: u32 = value.try_into().unwrap();
        assert_eq!(content, input);
    }

    #[test]
    fn value_from_u64() {
        let input = 1u64;
        let value = Value::from(input);
        let content: u64 = value.try_into().unwrap();
        assert_eq!(content, input);
    }

    #[test]
    fn value_from_command() {
        let input = Command::Call;
        let value = Value::from(input);
        let content: Command = value.try_into().unwrap();
        assert_eq!(content, input);
    }

    #[test]
    fn value_from_named() {
        let input = Named { name: Value::empty(), value: Value::empty() };
        let value = Value::from(input.clone());
        let content: &Named = (&value).try_into().unwrap();
        assert_eq!(content, &input);
    }

    #[test]
    fn value_from_bytes() {
        let input = vec![1u8, 2u8, 3u8];
        let value = Value::from(input.clone());
        let content: &Bytes = (&value).try_into().unwrap();
        assert_eq!(content, &input);
    }

    #[test]
    fn value_from_list() {
        let input = vec![Value::empty(), Value::empty()];
        let value = Value::from(input.clone());
        let content: &List = (&value).try_into().unwrap();
        assert_eq!(content, &input);
    }
}
