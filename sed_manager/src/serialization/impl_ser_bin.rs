use super::serialize::{Deserialize, Serialize};
use super::stream::{InputStream, ItemWrite, OutputStream};
use super::ItemRead;
use super::error::SerializeError;

macro_rules! impl_serialize_for_int {
    ($int_ty:ty) => {
        impl Serialize<$int_ty, u8> for $int_ty {
            type Error = SerializeError;
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
    type Error = SerializeError;
    fn serialize(&self, stream: &mut OutputStream<u8>) -> Result<(), Self::Error> {
        let byte = if *self { 1_u8 } else { 0_u8 };
        stream.write_one(byte);
        Ok(())
    }
}

macro_rules! impl_deserialize_for_int {
    ($int_ty:ty) => {
        impl Deserialize<$int_ty, u8> for $int_ty {
            type Error = SerializeError;
            fn deserialize(stream: &mut InputStream<u8>) -> Result<$int_ty, Self::Error> {
                let mut partial = [0_u8; size_of::<$int_ty>()];
                let mut num_bytes_read = 0;
                for byte in &mut partial {
                    if let Some(read_byte) = stream.read_one() {
                        *byte = *read_byte;
                        num_bytes_read += 1;
                    }
                }
                if num_bytes_read == 0 {
                    Err(SerializeError::EndOfStream)
                } else {
                    partial.rotate_left(num_bytes_read);
                    Ok(<$int_ty>::from_be_bytes(partial))
                }
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
    type Error = SerializeError;
    fn deserialize(stream: &mut InputStream<u8>) -> Result<bool, Self::Error> {
        let Some(byte) = stream.read_one() else {
            return Err(SerializeError::EndOfStream);
        };
        match byte {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(SerializeError::InvalidRepresentation),
        }
    }
}

impl<T, const LEN: usize> Serialize<[T; LEN], u8> for [T; LEN]
where
    T: Serialize<T, u8>,
    SerializeError: From<<T as Serialize<T, u8>>::Error>,
{
    type Error = SerializeError;
    fn serialize(&self, stream: &mut OutputStream<u8>) -> Result<(), Self::Error> {
        for item in self {
            match item.serialize(stream) {
                Ok(_) => (),
                Err(err) => return Err(err.into()),
            }
        }
        Ok(())
    }
}

impl<T, const LEN: usize> Deserialize<[T; LEN], u8> for [T; LEN]
where
    T: Deserialize<T, u8>,
    SerializeError: From<<T as Deserialize<T, u8>>::Error>,
{
    type Error = SerializeError;
    fn deserialize(stream: &mut InputStream<u8>) -> Result<[T; LEN], Self::Error> {
        let deserialize_one = |_| -> Result<T, Self::Error> { Ok(T::deserialize(stream)?) };

        let result: Result<Vec<_>, Self::Error> = (0..LEN).map(deserialize_one).collect();
        match result {
            Ok(items) => Ok(items.try_into().unwrap_or_else(|_| panic!("vector must be the right size at this point"))),
            Err(err) => Err(err),
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

    #[test]
    fn deserialize_partial() {
        let mut is = InputStream::<u8>::from(vec![0xFF, 0xFF]);
        let value = u64::deserialize(&mut is).unwrap();
        assert_eq!(value, 0x0000_0000_0000_FFFF_u64);
    }

    #[test]
    fn deserialize_leftover() {
        let mut is = InputStream::<u8>::from(vec![0xFF, 0xFF, 0x00, 0x00]);
        let value = u16::deserialize(&mut is).unwrap();
        assert_eq!(value, 0xFFFF_u16);
    }
}
