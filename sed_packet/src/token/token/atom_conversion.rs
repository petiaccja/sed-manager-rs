use std::marker::PhantomData;

use sorbit::bit::{PackInto, UnpackFrom};

use super::{Atom, LongAtom, MediumAtom, ShortAtom, TinyAtom};

macro_rules! impl_tiny_atom_try_from_int {
    ($int:ty, $signed:expr) => {
        impl TryFrom<$int> for TinyAtom {
            type Error = $int;

            fn try_from(value: $int) -> Result<Self, Self::Error> {
                match value.pack_into(6) {
                    Some(data) => Ok(TinyAtom { signed: $signed, data }),
                    None => Err(value),
                }
            }
        }
    };
}

macro_rules! impl_tiny_atom_try_into_int {
    ($int:ty, $signed:expr) => {
        impl<'a> TryFrom<&'a TinyAtom> for $int {
            type Error = &'a TinyAtom;

            fn try_from(value: &'a TinyAtom) -> Result<Self, Self::Error> {
                if value.signed == $signed {
                    match <$int>::unpack_from(value.data, 6) {
                        Ok(output) => Ok(output),
                        Err(_) => Err(value),
                    }
                } else if value.data & 0b1110_0000 == 0 {
                    match <$int>::unpack_from(value.data, 6) {
                        Ok(output) => Ok(output),
                        Err(_) => Err(value),
                    }
                } else {
                    Err(value)
                }
            }
        }
    };
}

impl_tiny_atom_try_from_int!(i8, true);
impl_tiny_atom_try_from_int!(i16, true);
impl_tiny_atom_try_from_int!(i32, true);
impl_tiny_atom_try_from_int!(i64, true);
impl_tiny_atom_try_from_int!(u8, false);
impl_tiny_atom_try_from_int!(u16, false);
impl_tiny_atom_try_from_int!(u32, false);
impl_tiny_atom_try_from_int!(u64, false);

impl_tiny_atom_try_into_int!(i8, true);
impl_tiny_atom_try_into_int!(i16, true);
impl_tiny_atom_try_into_int!(i32, true);
impl_tiny_atom_try_into_int!(i64, true);
impl_tiny_atom_try_into_int!(u8, false);
impl_tiny_atom_try_into_int!(u16, false);
impl_tiny_atom_try_into_int!(u32, false);
impl_tiny_atom_try_into_int!(u64, false);

macro_rules! impl_from_int {
    ($atom:ty, $int:ty, $signed:expr) => {
        impl From<$int> for $atom {
            fn from(value: $int) -> Self {
                Self {
                    tag: PhantomData,
                    byte: false,
                    signed: $signed,
                    length: PhantomData,
                    data: value.to_be_bytes().into(),
                }
            }
        }
    };
}

macro_rules! impl_try_into_int {
    ($atom:ty, $int:ty, $signed:expr) => {
        impl<'a> TryFrom<&'a $atom> for $int {
            type Error = &'a $atom;

            fn try_from(value: &'a $atom) -> Result<Self, Self::Error> {
                if value.signed == $signed && !value.byte {
                    let resized = resize_integer::<{ size_of::<$int>() }>(&value.data, value.signed).ok_or(value)?;
                    Ok(<$int>::from_be_bytes(resized))
                } else if !value.byte
                    && (value.data.first().is_some_and(|byte| byte & 0b1000_0000 == 0) || value.data.is_empty())
                {
                    let resized = resize_integer::<{ size_of::<$int>() }>(&value.data, value.signed).ok_or(value)?;
                    Ok(<$int>::from_be_bytes(resized))
                } else {
                    Err(value)
                }
            }
        }
    };
}

fn resize_integer<const NUM_BYTES: usize>(bytes: &[u8], signed: bool) -> Option<[u8; NUM_BYTES]> {
    // Adding or truncating leading bytes won't change the overall value of the integer.
    // For unsigned value, this is always 0x00.
    // For negative signed values, the leading bytes are 0xFF in two's complement.
    // For non-negative signed values, the leading bytes are 0x00 like for unsigned.
    let leading_byte = if signed {
        match bytes.first().cloned().unwrap_or(0).leading_ones() {
            0 => 0x00,
            _ => 0xFF,
        }
    } else {
        0x00
    };

    if NUM_BYTES <= bytes.len() {
        // Need to truncate.
        let num_truncated = bytes.len() - NUM_BYTES;
        if bytes[0..num_truncated].iter().all(|byte| *byte == leading_byte) {
            let mut truncated = [0; NUM_BYTES];
            truncated.copy_from_slice(&bytes[num_truncated..]);
            Some(truncated)
        } else {
            None
        }
    } else {
        // Need to extend.
        let num_extended = NUM_BYTES - bytes.len();
        let mut extended = [leading_byte; NUM_BYTES];
        extended[num_extended..].copy_from_slice(bytes);
        Some(extended)
    }
}

impl_from_int!(ShortAtom, i8, true);
impl_from_int!(ShortAtom, i16, true);
impl_from_int!(ShortAtom, i32, true);
impl_from_int!(ShortAtom, i64, true);
impl_from_int!(ShortAtom, u8, false);
impl_from_int!(ShortAtom, u16, false);
impl_from_int!(ShortAtom, u32, false);
impl_from_int!(ShortAtom, u64, false);

impl_try_into_int!(ShortAtom, i8, true);
impl_try_into_int!(ShortAtom, i16, true);
impl_try_into_int!(ShortAtom, i32, true);
impl_try_into_int!(ShortAtom, i64, true);
impl_try_into_int!(ShortAtom, u8, false);
impl_try_into_int!(ShortAtom, u16, false);
impl_try_into_int!(ShortAtom, u32, false);
impl_try_into_int!(ShortAtom, u64, false);

impl_from_int!(MediumAtom, i8, true);
impl_from_int!(MediumAtom, i16, true);
impl_from_int!(MediumAtom, i32, true);
impl_from_int!(MediumAtom, i64, true);
impl_from_int!(MediumAtom, u8, false);
impl_from_int!(MediumAtom, u16, false);
impl_from_int!(MediumAtom, u32, false);
impl_from_int!(MediumAtom, u64, false);

impl_try_into_int!(MediumAtom, i8, true);
impl_try_into_int!(MediumAtom, i16, true);
impl_try_into_int!(MediumAtom, i32, true);
impl_try_into_int!(MediumAtom, i64, true);
impl_try_into_int!(MediumAtom, u8, false);
impl_try_into_int!(MediumAtom, u16, false);
impl_try_into_int!(MediumAtom, u32, false);
impl_try_into_int!(MediumAtom, u64, false);

impl_from_int!(LongAtom, i8, true);
impl_from_int!(LongAtom, i16, true);
impl_from_int!(LongAtom, i32, true);
impl_from_int!(LongAtom, i64, true);
impl_from_int!(LongAtom, u8, false);
impl_from_int!(LongAtom, u16, false);
impl_from_int!(LongAtom, u32, false);
impl_from_int!(LongAtom, u64, false);

impl_try_into_int!(LongAtom, i8, true);
impl_try_into_int!(LongAtom, i16, true);
impl_try_into_int!(LongAtom, i32, true);
impl_try_into_int!(LongAtom, i64, true);
impl_try_into_int!(LongAtom, u8, false);
impl_try_into_int!(LongAtom, u16, false);
impl_try_into_int!(LongAtom, u32, false);
impl_try_into_int!(LongAtom, u64, false);

macro_rules! impl_try_from_bytes {
    ($atom:ty) => {
        impl<'a> TryFrom<&'a [u8]> for $atom {
            type Error = &'a [u8];

            fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
                if let Self::DATA_LEN_MIN..=Self::DATA_LEN_MAX = value.len() {
                    Ok(Self { tag: PhantomData, byte: true, signed: false, length: PhantomData, data: value.into() })
                } else {
                    Err(value)
                }
            }
        }

        impl TryFrom<Vec<u8>> for $atom {
            type Error = Vec<u8>;

            fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
                if let Self::DATA_LEN_MIN..=Self::DATA_LEN_MAX = value.len() {
                    Ok(Self { tag: PhantomData, byte: true, signed: false, length: PhantomData, data: value.into() })
                } else {
                    Err(value)
                }
            }
        }
    };
}

macro_rules! impl_try_into_bytes {
    ($atom:ty) => {
        impl<'a> TryFrom<&'a $atom> for &'a [u8] {
            type Error = &'a $atom;

            fn try_from(value: &'a $atom) -> Result<Self, Self::Error> {
                if value.byte { Ok(&value.data) } else { Err(value) }
            }
        }

        impl<'a> TryFrom<&'a $atom> for Vec<u8> {
            type Error = &'a $atom;

            fn try_from(value: &'a $atom) -> Result<Self, Self::Error> {
                if value.byte { Ok(value.data.clone()) } else { Err(value) }
            }
        }

        impl TryFrom<$atom> for Vec<u8> {
            type Error = $atom;

            fn try_from(value: $atom) -> Result<Self, Self::Error> {
                if value.byte { Ok(value.data) } else { Err(value) }
            }
        }
    };
}

impl_try_from_bytes!(ShortAtom);
impl_try_from_bytes!(MediumAtom);
impl_try_from_bytes!(LongAtom);

impl_try_into_bytes!(ShortAtom);
impl_try_into_bytes!(MediumAtom);
impl_try_into_bytes!(LongAtom);

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use super::*;

    #[test]
    fn tiny_atom_from_int() {
        assert_eq!(TinyAtom::try_from(64u32), Err(64));
        assert_eq!(TinyAtom::try_from(63u32), Ok(TinyAtom { signed: false, data: 63 }));
        assert_eq!(TinyAtom::try_from(32i32), Err(32));
        assert_eq!(TinyAtom::try_from(31i32), Ok(TinyAtom { signed: true, data: 31 }));
    }

    #[test]
    fn tiny_atom_into_int() {
        assert_eq!(i32::try_from(&TinyAtom { signed: false, data: 63 }), Err(&TinyAtom { signed: false, data: 63 }));
        assert_eq!(i32::try_from(&TinyAtom { signed: false, data: 3 }), Ok(3));
        assert_eq!(i32::try_from(&TinyAtom { signed: true, data: 3 }), Ok(3));

        assert_eq!(u32::try_from(&TinyAtom { signed: false, data: 3 }), Ok(3));
        assert_eq!(u32::try_from(&TinyAtom { signed: true, data: 3 }), Ok(3));
        assert_eq!(
            u32::try_from(&TinyAtom { signed: true, data: 0b0010_0000 }),
            Err(&TinyAtom { signed: true, data: 0b0010_0000 })
        );
    }

    #[test]
    fn bigger_atoms_from_int() {
        assert_eq!(
            MediumAtom::from(63u16),
            MediumAtom { tag: PhantomData, signed: false, byte: false, length: PhantomData, data: vec![0, 63] }
        );
        assert_eq!(
            MediumAtom::from(31i16),
            MediumAtom { tag: PhantomData, signed: true, byte: false, length: PhantomData, data: vec![0, 31] }
        );
    }

    #[test]
    fn bigger_atoms_into_int_mixed_sign() {
        // Unsigned to signed success
        assert_eq!(
            i8::try_from(&MediumAtom {
                tag: PhantomData,
                signed: false,
                byte: false,
                length: PhantomData,
                data: vec![127]
            }),
            Ok(127)
        );

        // Unsigned to signed overflow
        assert!(
            i8::try_from(&MediumAtom {
                tag: PhantomData,
                signed: false,
                byte: false,
                length: PhantomData,
                data: vec![128],
            })
            .is_err(),
        );

        // Signed to unsigned success
        assert_eq!(
            u8::try_from(&MediumAtom {
                tag: PhantomData,
                signed: true,
                byte: false,
                length: PhantomData,
                data: vec![127]
            }),
            Ok(127)
        );

        // Unsigned to signed overflow
        assert!(
            u8::try_from(&MediumAtom {
                tag: PhantomData,
                signed: true,
                byte: false,
                length: PhantomData,
                data: vec![128],
            })
            .is_err(),
        );
    }

    #[test]
    fn bigger_atoms_into_int_truncate() {
        // Signed negative success.
        assert_eq!(
            i8::try_from(&MediumAtom {
                tag: PhantomData,
                signed: true,
                byte: false,
                length: PhantomData,
                data: vec![255, 255, 255]
            }),
            Ok(-1)
        );

        // Signed negative failure.
        assert!(
            i8::try_from(&MediumAtom {
                tag: PhantomData,
                signed: true,
                byte: false,
                length: PhantomData,
                data: vec![254, 255, 255],
            })
            .is_err(),
        );

        // Signed positive success.
        assert_eq!(
            i8::try_from(&MediumAtom {
                tag: PhantomData,
                signed: true,
                byte: false,
                length: PhantomData,
                data: vec![0, 0, 127]
            }),
            Ok(127)
        );

        // Signed positive sign conversion success.
        assert_eq!(
            i8::try_from(&MediumAtom {
                tag: PhantomData,
                signed: false,
                byte: false,
                length: PhantomData,
                data: vec![0, 0, 127]
            }),
            Ok(127)
        );

        // Signed positive failure.
        assert!(
            i8::try_from(&MediumAtom {
                tag: PhantomData,
                signed: true,
                byte: false,
                length: PhantomData,
                data: vec![1, 0, 127],
            })
            .is_err(),
        );

        // Unsigned success.
        assert_eq!(
            u8::try_from(&MediumAtom {
                tag: PhantomData,
                signed: false,
                byte: false,
                length: PhantomData,
                data: vec![0, 0, 127]
            }),
            Ok(127)
        );

        // Unsigned sign conversion success.
        assert_eq!(
            u8::try_from(&MediumAtom {
                tag: PhantomData,
                signed: true,
                byte: false,
                length: PhantomData,
                data: vec![0, 0, 127]
            }),
            Ok(127)
        );

        // Unsigned failure.
        assert!(
            u8::try_from(&MediumAtom {
                tag: PhantomData,
                signed: false,
                byte: false,
                length: PhantomData,
                data: vec![1, 0, 127],
            })
            .is_err(),
        );
    }

    #[test]
    fn bigger_atoms_into_int_extend() {
        // Signed negative success.
        assert_eq!(
            i32::try_from(&MediumAtom {
                tag: PhantomData,
                signed: true,
                byte: false,
                length: PhantomData,
                data: vec![255, 255, 255]
            }),
            Ok(-1)
        );

        // Signed positive success.
        assert_eq!(
            i32::try_from(&MediumAtom {
                tag: PhantomData,
                signed: true,
                byte: false,
                length: PhantomData,
                data: vec![0, 0, 127]
            }),
            Ok(127)
        );

        // Signed positive sign conversion success.
        assert_eq!(
            i32::try_from(&MediumAtom {
                tag: PhantomData,
                signed: false,
                byte: false,
                length: PhantomData,
                data: vec![0, 0, 127]
            }),
            Ok(127)
        );

        // Unsigned success.
        assert_eq!(
            u32::try_from(&MediumAtom {
                tag: PhantomData,
                signed: false,
                byte: false,
                length: PhantomData,
                data: vec![0, 0, 127]
            }),
            Ok(127)
        );

        // Unsigned sign conversion success.
        assert_eq!(
            u32::try_from(&MediumAtom {
                tag: PhantomData,
                signed: true,
                byte: false,
                length: PhantomData,
                data: vec![0, 0, 127]
            }),
            Ok(127)
        );
    }

    #[test]
    fn from_bytes_fitting() {
        let bytes = vec![15];
        assert_eq!(
            ShortAtom::try_from(bytes.as_slice()),
            Ok(ShortAtom { tag: PhantomData, byte: true, signed: false, length: PhantomData, data: bytes.clone() })
        );

        assert_eq!(
            ShortAtom::try_from(bytes.clone()),
            Ok(ShortAtom { tag: PhantomData, byte: true, signed: false, length: PhantomData, data: bytes.clone() })
        );
    }

    #[test]
    fn from_bytes_overflow() {
        let bytes = vec![0; 16];
        assert!(ShortAtom::try_from(bytes.as_slice()).is_err());
        assert!(ShortAtom::try_from(bytes.clone()).is_err());
    }

    #[test]
    fn into_bytes_is_byte() {
        let atom = ShortAtom { tag: PhantomData, byte: true, signed: false, length: PhantomData, data: vec![1, 2, 3] };
        assert_eq!(<&[u8]>::try_from(&atom), Ok([1, 2, 3].as_slice()));
        assert_eq!(<Vec<u8>>::try_from(&atom), Ok(vec![1, 2, 3]));
        assert_eq!(<Vec<u8>>::try_from(atom), Ok(vec![1, 2, 3]));
    }

    #[test]
    fn into_bytes_is_not_byte() {
        let atom = ShortAtom { tag: PhantomData, byte: false, signed: false, length: PhantomData, data: vec![1, 2, 3] };
        assert!(<&[u8]>::try_from(&atom).is_err());
        assert!(<Vec<u8>>::try_from(&atom).is_err());
        assert!(<Vec<u8>>::try_from(atom).is_err());
    }
}
