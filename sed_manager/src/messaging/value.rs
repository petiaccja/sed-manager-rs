//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

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

#[derive(Clone, PartialEq, Eq)]
pub enum Value {
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

//------------------------------------------------------------------------------
// Implementations for Value.
//------------------------------------------------------------------------------

impl Value {
    pub fn empty() -> Self {
        Self::Empty
    }

    pub fn is_empty(&self) -> bool {
        match &self {
            Self::Empty => true,
            _ => false,
        }
    }
    pub fn is_empty_command(&self) -> bool {
        match &self {
            Self::Command(Command::Empty) => true,
            _ => false,
        }
    }
}

impl core::fmt::Debug for Value {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> Result<(), core::fmt::Error> {
        match self {
            Value::Empty => f.write_str("<EMPTY>"),
            Value::Int8(n) => write!(f, "{n}_u8"),
            Value::Int16(n) => write!(f, "{n}_i16"),
            Value::Int32(n) => write!(f, "{n}_i32"),
            Value::Int64(n) => write!(f, "{n}_i64"),
            Value::Uint8(n) => write!(f, "{n}_u8"),
            Value::Uint16(n) => write!(f, "{n}_u16"),
            Value::Uint32(n) => write!(f, "{n}_u32"),
            Value::Uint64(n) => write!(f, "{n}_u64"),
            Value::Command(command) => {
                if f.alternate() {
                    write!(f, "{command:#?}")
                } else {
                    write!(f, "{command:?}")
                }
            }
            Value::Named(named) => {
                if f.alternate() {
                    write!(f, "{named:#?}")
                } else {
                    write!(f, "{named:?}")
                }
            }
            Value::Bytes(bytes) => {
                f.write_str("bytes")?;
                f.debug_list().entries(bytes.iter()).finish()
            }
            Value::List(values) => {
                f.write_str("list")?;
                f.debug_list().entries(values.iter()).finish()
            }
        }
    }
}

//------------------------------------------------------------------------------
// From trait implementation macros.
//------------------------------------------------------------------------------

macro_rules! value_from_type {
    ($storage_ty:ty, $enum_variant:expr) => {
        impl From<$storage_ty> for Value {
            fn from(value: $storage_ty) -> Self {
                $enum_variant(value.into())
            }
        }
    };
}

macro_rules! type_from_value {
    { $storage_ty:ty, $value_expr:expr, $($enum_variants:pat),+} => {
        impl TryFrom<Value> for $storage_ty {
            type Error = Value;
            fn try_from(value: Value) -> Result<Self, Self::Error> {
                match value {
                    $($enum_variants => Ok($value_expr),)+
                    _ => Err(value),
                }
            }
        }

        impl<'value> TryFrom<&'value Value> for $storage_ty {
            type Error = &'value Value;
            fn try_from(value: &'value Value) -> Result<Self, Self::Error> {
                match value {
                    $($enum_variants => Ok($value_expr),)+
                    _ => Err(value),
                }
            }
        }
    };
}

//------------------------------------------------------------------------------
// Value from type implementations.
//------------------------------------------------------------------------------

value_from_type!(bool, Self::Uint8);
value_from_type!(i8, Self::Int8);
value_from_type!(i16, Self::Int16);
value_from_type!(i32, Self::Int32);
value_from_type!(i64, Self::Int64);
value_from_type!(u8, Self::Uint8);
value_from_type!(u16, Self::Uint16);
value_from_type!(u32, Self::Uint32);
value_from_type!(u64, Self::Uint64);
value_from_type!(Command, Self::Command);
value_from_type!(Bytes, Self::Bytes);
value_from_type!(List, Self::List);

impl From<Named> for Value {
    fn from(value: Named) -> Self {
        Self::Named(Box::new(value))
    }
}

impl<const N: usize> From<[u8; N]> for Value {
    fn from(value: [u8; N]) -> Self {
        Self::Bytes(Bytes::from(value))
    }
}

impl From<&[u8]> for Value {
    fn from(value: &[u8]) -> Self {
        Self::Bytes(Bytes::from(value))
    }
}

//------------------------------------------------------------------------------
// Type from value implementations.
//------------------------------------------------------------------------------

type_from_value!(i8, value.clone().into(), Value::Int8(value));
type_from_value!(i16, value.clone().into(), Value::Int8(value), Value::Int16(value), Value::Uint8(value));
type_from_value!(
    i32,
    value.clone().into(),
    Value::Int8(value),
    Value::Int16(value),
    Value::Int32(value),
    Value::Uint8(value),
    Value::Uint16(value)
);
type_from_value!(
    i64,
    value.clone().into(),
    Value::Int8(value),
    Value::Int16(value),
    Value::Int32(value),
    Value::Int64(value),
    Value::Uint8(value),
    Value::Uint16(value),
    Value::Uint32(value)
);
type_from_value!(u8, value.clone().into(), Value::Uint8(value));
type_from_value!(u16, value.clone().into(), Value::Uint8(value), Value::Uint16(value));
type_from_value!(u32, value.clone().into(), Value::Uint8(value), Value::Uint16(value), Value::Uint32(value));
type_from_value!(
    u64,
    value.clone().into(),
    Value::Uint8(value),
    Value::Uint16(value),
    Value::Uint32(value),
    Value::Uint64(value)
);
type_from_value!(Command, value.clone(), Value::Command(value));

impl TryFrom<Value> for bool {
    type Error = Value;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match Self::try_from(&value) {
            Ok(x) => Ok(x),
            Err(_) => Err(value),
        }
    }
}

impl<'value> TryFrom<&'value Value> for bool {
    type Error = &'value Value;
    fn try_from(value: &'value Value) -> Result<Self, Self::Error> {
        let n = match value {
            Value::Int8(n) => Ok(*n as i64),
            Value::Int16(n) => Ok(*n as i64),
            Value::Int32(n) => Ok(*n as i64),
            Value::Int64(n) => Ok(*n as i64),
            Value::Uint8(n) => Ok(*n as i64),
            Value::Uint16(n) => Ok(*n as i64),
            Value::Uint32(n) => Ok(*n as i64),
            Value::Uint64(n) => Ok(*n as i64),
            _ => Err(value),
        }?;
        match n {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(value),
        }
    }
}

impl TryFrom<Value> for Named {
    type Error = Value;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Named(value) => Ok(*value),
            _ => Err(value),
        }
    }
}

impl TryFrom<Value> for Bytes {
    type Error = Value;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Bytes(value) => Ok(value),
            _ => Err(value),
        }
    }
}

impl<const N: usize> TryFrom<Value> for [u8; N] {
    type Error = Value;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Bytes(value) => match Self::try_from(value) {
                Ok(array) => Ok(array),
                Err(value) => Err(Value::from(value)),
            },
            _ => Err(value),
        }
    }
}

impl TryFrom<Value> for List {
    type Error = Value;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::List(value) => Ok(value),
            _ => Err(value),
        }
    }
}

impl<'value> TryFrom<&'value Value> for &'value Named {
    type Error = &'value Value;
    fn try_from(value: &'value Value) -> Result<Self, Self::Error> {
        match value {
            Value::Named(ref value) => Ok(&*value),
            _ => Err(value),
        }
    }
}

impl<'value> TryFrom<&'value Value> for &'value Bytes {
    type Error = &'value Value;
    fn try_from(value: &'value Value) -> Result<Self, Self::Error> {
        match value {
            Value::Bytes(ref value) => Ok(value),
            _ => Err(value),
        }
    }
}

impl<'value> TryFrom<&'value Value> for &'value [u8] {
    type Error = &'value Value;
    fn try_from(value: &'value Value) -> Result<Self, Self::Error> {
        match value {
            Value::Bytes(ref items) => match Self::try_from(items.as_slice()) {
                Ok(array) => Ok(array),
                Err(_) => Err(value),
            },
            _ => Err(value),
        }
    }
}

impl<'value, const N: usize> TryFrom<&'value Value> for &'value [u8; N] {
    type Error = &'value Value;
    fn try_from(value: &'value Value) -> Result<Self, Self::Error> {
        match value {
            Value::Bytes(ref items) => match Self::try_from(items.as_slice()) {
                Ok(array) => Ok(array),
                Err(_) => Err(value),
            },
            _ => Err(value),
        }
    }
}

impl<'value> TryFrom<&'value Value> for &'value List {
    type Error = &'value Value;
    fn try_from(value: &'value Value) -> Result<Self, Self::Error> {
        match value {
            Value::List(ref value) => Ok(value),
            _ => Err(value),
        }
    }
}

impl<'value> TryFrom<&'value Value> for Named {
    type Error = &'value Value;
    fn try_from(value: &'value Value) -> Result<Self, Self::Error> {
        match value {
            Value::Named(ref value) => Ok(value.as_ref().clone()),
            _ => Err(value),
        }
    }
}

impl<'value> TryFrom<&'value Value> for Bytes {
    type Error = &'value Value;
    fn try_from(value: &'value Value) -> Result<Self, Self::Error> {
        match value {
            Value::Bytes(ref value) => Ok(value.clone()),
            _ => Err(value),
        }
    }
}

impl<'value, const N: usize> TryFrom<&'value Value> for [u8; N] {
    type Error = &'value Value;
    fn try_from(value: &'value Value) -> Result<Self, Self::Error> {
        match value {
            Value::Bytes(ref items) => match Self::try_from(items.as_slice()) {
                Ok(array) => Ok(array),
                Err(_) => Err(value),
            },
            _ => Err(value),
        }
    }
}

impl<'value> TryFrom<&'value Value> for List {
    type Error = &'value Value;
    fn try_from(value: &'value Value) -> Result<Self, Self::Error> {
        match value {
            Value::List(ref value) => Ok(value.clone()),
            _ => Err(value),
        }
    }
}

//------------------------------------------------------------------------------
// Unit tests.
//------------------------------------------------------------------------------

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

    #[test]
    fn format_value_list() {
        let input = Value::from(vec![Value::from(1), Value::from(2)]);
        assert_eq!(format!("{input:?}"), "list[1_i32, 2_i32]");
    }
}
