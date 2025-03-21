//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use super::error::Error;
use super::serialize::{Deserialize, Serialize};
use super::stream::{InputStream, ItemWrite, OutputStream};
use super::ByteOrder;
use super::ItemRead;

macro_rules! serialize_integer {
    ($int_ty:ty) => {
        impl Serialize<u8> for $int_ty {
            type Error = Error;
            fn serialize(&self, stream: &mut OutputStream<u8>) -> Result<(), Self::Error> {
                if stream.byte_order == ByteOrder::BigEndian {
                    stream.write_exact(&self.to_be_bytes());
                } else {
                    stream.write_exact(&self.to_le_bytes());
                }
                Ok(())
            }
        }
    };
}

macro_rules! deserialize_integer {
    ($int_ty:ty) => {
        impl Deserialize<u8> for $int_ty {
            type Error = Error;
            fn deserialize(stream: &mut InputStream<u8>) -> Result<Self, Self::Error> {
                let mut partial = [0_u8; size_of::<$int_ty>()];
                let mut num_bytes_read = 0;
                for byte in &mut partial {
                    if let Ok(read_byte) = stream.read_one() {
                        *byte = *read_byte;
                        num_bytes_read += 1;
                    }
                }
                if num_bytes_read == 0 {
                    Err(Error::EndOfStream)
                } else {
                    partial.rotate_left(num_bytes_read);
                    if stream.byte_order == ByteOrder::BigEndian {
                        Ok(<$int_ty>::from_be_bytes(partial))
                    } else {
                        Ok(<$int_ty>::from_le_bytes(partial))
                    }
                }
            }
        }
    };
}

serialize_integer!(u8);
serialize_integer!(u16);
serialize_integer!(u32);
serialize_integer!(u64);
serialize_integer!(i8);
serialize_integer!(i16);
serialize_integer!(i32);
serialize_integer!(i64);

deserialize_integer!(u8);
deserialize_integer!(u16);
deserialize_integer!(u32);
deserialize_integer!(u64);
deserialize_integer!(i8);
deserialize_integer!(i16);
deserialize_integer!(i32);
deserialize_integer!(i64);

impl Serialize<u8> for bool {
    type Error = Error;
    fn serialize(&self, stream: &mut OutputStream<u8>) -> Result<(), Self::Error> {
        let byte = if *self { 1_u8 } else { 0_u8 };
        stream.write_one(byte);
        Ok(())
    }
}

impl Deserialize<u8> for bool {
    type Error = Error;
    fn deserialize(stream: &mut InputStream<u8>) -> Result<Self, Self::Error> {
        let byte = stream.read_one()?;
        match byte {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Error::EndOfStream),
        }
    }
}

impl<T, const LEN: usize> Serialize<u8> for [T; LEN]
where
    T: Serialize<u8>,
    Error: From<<T as Serialize<u8>>::Error>,
{
    type Error = Error;
    fn serialize(&self, stream: &mut OutputStream<u8>) -> Result<(), Self::Error> {
        for item in self {
            item.serialize(stream)?;
        }
        Ok(())
    }
}

impl<T, const LEN: usize> Deserialize<u8> for [T; LEN]
where
    T: Deserialize<u8>,
    Error: From<<T as Deserialize<u8>>::Error>,
{
    type Error = Error;
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
    use crate::serialization::ByteOrder;

    use super::*;

    macro_rules! test_serialize {
        ($int_ty:ty, $name_be:ident, $name_le:ident) => {
            #[test]
            fn $name_be() {
                let init_data: [u8; size_of::<$int_ty>()] = core::array::from_fn(|idx| idx as u8);
                let input = <$int_ty>::from_be_bytes(init_data.clone());
                let mut os = OutputStream::<u8>::new();
                os.byte_order = ByteOrder::BigEndian;
                assert!(input.serialize(&mut os).is_ok());
                let buf = os.take();
                assert_eq!(&buf, &init_data);
                let mut is = InputStream::<u8>::from(buf);
                is.byte_order = ByteOrder::BigEndian;
                let value = <$int_ty>::deserialize(&mut is).unwrap();
                assert_eq!(input, value);
            }
            #[test]
            fn $name_le() {
                let init_data: [u8; size_of::<$int_ty>()] = core::array::from_fn(|idx| idx as u8);
                let input = <$int_ty>::from_le_bytes(init_data.clone());
                let mut os = OutputStream::<u8>::new();
                os.byte_order = ByteOrder::LittleEndian;
                assert!(input.serialize(&mut os).is_ok());
                let buf = os.take();
                assert_eq!(&buf, &init_data);
                let mut is = InputStream::<u8>::from(buf);
                is.byte_order = ByteOrder::LittleEndian;
                let value = <$int_ty>::deserialize(&mut is).unwrap();
                assert_eq!(input, value);
            }
        };
    }

    test_serialize!(u8, serialize_u8_be, serialize_u8_le);
    test_serialize!(u16, serialize_u16_be, serialize_u16_le);
    test_serialize!(u32, serialize_u32_be, serialize_u32_le);
    test_serialize!(u64, serialize_u64_be, serialize_u64_le);
    test_serialize!(i8, serialize_i8_be, serialize_i8_le);
    test_serialize!(i16, serialize_i16_be, serialize_i16_le);
    test_serialize!(i32, serialize_i32_be, serialize_i32_le);
    test_serialize!(i64, serialize_i64_be, serialize_i64_le);

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
