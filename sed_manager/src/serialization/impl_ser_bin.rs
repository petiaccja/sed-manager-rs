use super::serialize::{Deserialize, SerializationError, Serialize};
use super::stream::{InputStream, ItemWrite, OutputStream};
use super::ItemRead;

macro_rules! impl_serialize_for_int {
    ($int_ty:ty) => {
        impl Serialize<$int_ty, u8> for $int_ty {
            type Error = SerializationError;
            fn serialize(&self, stream: &mut OutputStream<u8>) -> Result<(), Self::Error> {
                stream.write_exact(&self.to_be_bytes());
                Ok(())
            }
        }
    };
}

impl_serialize_for_int!(u8);
impl_serialize_for_int!(u16);
impl_serialize_for_int!(u32);
impl_serialize_for_int!(u64);
impl_serialize_for_int!(i8);
impl_serialize_for_int!(i16);
impl_serialize_for_int!(i32);
impl_serialize_for_int!(i64);

impl Serialize<bool, u8> for bool {
    type Error = SerializationError;
    fn serialize(&self, stream: &mut OutputStream<u8>) -> Result<(), Self::Error> {
        let byte = if *self { 1_u8 } else { 0_u8 };
        stream.write_one(byte);
        Ok(())
    }
}

macro_rules! impl_deserialize_for_int {
    ($int_ty:ty) => {
        impl Deserialize<$int_ty, u8> for $int_ty {
            type Error = SerializationError;
            fn deserialize(stream: &mut InputStream<u8>) -> Result<$int_ty, Self::Error> {
                let Some(bytes) = stream.read_exact(size_of::<$int_ty>()) else {
                    return Err(SerializationError::EndOfStream);
                };
                Ok(<$int_ty>::from_be_bytes(bytes.try_into().unwrap()))
            }
        }
    };
}

impl_deserialize_for_int!(u8);
impl_deserialize_for_int!(u16);
impl_deserialize_for_int!(u32);
impl_deserialize_for_int!(u64);
impl_deserialize_for_int!(i8);
impl_deserialize_for_int!(i16);
impl_deserialize_for_int!(i32);
impl_deserialize_for_int!(i64);

impl Deserialize<bool, u8> for bool {
    type Error = SerializationError;
    fn deserialize(stream: &mut InputStream<u8>) -> Result<bool, Self::Error> {
        let Some(byte) = stream.read_one() else {
            return Err(SerializationError::EndOfStream);
        };
        match byte {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(SerializationError::Overflow),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_serialize {
        ($int_ty:ty, $name:ident) => {
            #[test]
            fn $name() {
                let input = <$int_ty>::max_value();
                let mut os = OutputStream::<u8>::new();
                assert!(input.serialize(&mut os).is_ok());
                let mut is = InputStream::<u8>::from(os.take());
                let value = <$int_ty>::deserialize(&mut is).unwrap();
                assert_eq!(input, value);
            }
        };
    }

    test_serialize!(u8, serialize_u8);
    test_serialize!(u16, serialize_u16);
    test_serialize!(u32, serialize_u32);
    test_serialize!(u64, serialize_u64);
    test_serialize!(i8, serialize_i8);
    test_serialize!(i16, serialize_i16);
    test_serialize!(i32, serialize_i32);
    test_serialize!(i64, serialize_i64);

    #[test]
    fn serialize_bool() {
        for input in [true, false] {
            let mut os = OutputStream::<u8>::new();
            assert!(input.serialize(&mut os).is_ok());
            let mut is = InputStream::<u8>::from(os.take());
            let value = bool::deserialize(&mut is).unwrap();
            assert_eq!(input, value);
        }
    }
}
