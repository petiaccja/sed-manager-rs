mod atom_conversion;
mod token_conversion;

use std::marker::PhantomData;

use sorbit::collection::deserialize_items_by_len;
use sorbit::error::MessageError as _;
use sorbit::ser_de::{Deserialize, FromBytes, Serialize};
use sorbit::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum Token {
    TinyAtom(TinyAtom),
    ShortAtom(ShortAtom),
    MediumAtom(MediumAtom),
    LongAtom(LongAtom),
    StartList,
    EndList,
    StartName,
    EndName,
    Call,
    EndOfData,
    EndOfSession,
    StartTransaction,
    EndTransaction,
    Empty,
}

#[repr(u8)]
enum Tag {
    TinyAtom = 0b0000_0000,
    ShortAtom = 0b1000_0000,
    MediumAtom = 0b1100_0000,
    LongAtom = 0b1110_0000,
    StartList = 0xF0,
    EndList = 0xF1,
    StartName = 0xF2,
    EndName = 0xF3,
    Call = 0xF8,
    EndOfData = 0xF9,
    EndOfSession = 0xFA,
    StartTransaction = 0xFB,
    EndTransaction = 0xFC,
    Empty = 0xFF,
}

trait Atom {
    const DATA_LEN_MIN: usize;
    const DATA_LEN_MAX: usize;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[sorbit(byte_order=big_endian)]
pub struct TinyAtom {
    #[sorbit(bit_field=_0, repr=u8, bits=6)]
    pub signed: bool,
    #[sorbit(bit_field=_0, bits=0..6)]
    pub data: u8,
}

impl Atom for TinyAtom {
    const DATA_LEN_MIN: usize = 0;
    const DATA_LEN_MAX: usize = 0;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[sorbit(byte_order=big_endian)]
pub struct ShortAtom {
    #[sorbit(bit_field=_0, repr=u8, bits=6..8, value=constant(Tag::ShortAtom as u8 >> 6))]
    pub tag: PhantomData<u8>,
    #[sorbit(bit_field=_0, bits=5)]
    pub byte: bool,
    #[sorbit(bit_field=_0, bits=4)]
    pub signed: bool,
    #[sorbit(bit_field=_0, bits=0..4, value=len(data))]
    pub length: PhantomData<u8>,
    pub data: Vec<u8>,
}

impl Atom for ShortAtom {
    const DATA_LEN_MIN: usize = 0;
    const DATA_LEN_MAX: usize = 15;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[sorbit(byte_order=big_endian)]
pub struct MediumAtom {
    #[sorbit(bit_field=_0, repr=u16, bits=13..16, value=constant(Tag::MediumAtom as u8 >> 5))]
    pub tag: PhantomData<u8>,
    #[sorbit(bit_field=_0, bits=12)]
    pub byte: bool,
    #[sorbit(bit_field=_0, bits=11)]
    pub signed: bool,
    #[sorbit(bit_field=_0, bits=0..11, value=len(data))]
    pub length: PhantomData<u16>,
    pub data: Vec<u8>,
}

impl Atom for MediumAtom {
    const DATA_LEN_MIN: usize = 1;
    const DATA_LEN_MAX: usize = 2047;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[sorbit(byte_order=big_endian)]
pub struct LongAtom {
    #[sorbit(bit_field=_0, repr=u32, bits=28..32, value=constant(Tag::LongAtom as u8 >> 4))]
    pub tag: PhantomData<u8>,
    #[sorbit(bit_field=_0, bits=25)]
    pub byte: bool,
    #[sorbit(bit_field=_0, bits=24)]
    pub signed: bool,
    #[sorbit(bit_field=_0, bits=0..24, value=len(data))]
    pub length: PhantomData<u32>,
    pub data: Vec<u8>,
}

impl Atom for LongAtom {
    const DATA_LEN_MIN: usize = 1;
    const DATA_LEN_MAX: usize = 16777215;
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[sorbit(byte_order=big_endian)]
struct ShortAtomHeader {
    #[sorbit(bit_field=_0, repr=u8, bits=6..8, value=constant(Tag::ShortAtom as u8 >> 6))]
    tag: PhantomData<u8>,
    #[sorbit(bit_field=_0, bits=5)]
    byte: bool,
    #[sorbit(bit_field=_0, bits=4)]
    signed: bool,
    #[sorbit(bit_field=_0, bits=0..4)]
    length: u8,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[sorbit(byte_order=big_endian)]
struct MediumAtomHeader {
    #[sorbit(bit_field=_0, repr=u16, bits=13..16, value=constant(Tag::MediumAtom as u8 >> 5))]
    tag: PhantomData<u8>,
    #[sorbit(bit_field=_0, bits=12)]
    byte: bool,
    #[sorbit(bit_field=_0, bits=11)]
    signed: bool,
    #[sorbit(bit_field=_0, bits=0..11)]
    length: u16,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[sorbit(byte_order=big_endian)]
struct LongAtomHeader {
    #[sorbit(bit_field=_0, repr=u32, bits=28..32, value=constant(Tag::LongAtom as u8 >> 4))]
    tag: PhantomData<u8>,
    #[sorbit(bit_field=_0, bits=25)]
    byte: bool,
    #[sorbit(bit_field=_0, bits=24)]
    signed: bool,
    #[sorbit(bit_field=_0, bits=0..24)]
    length: u32,
}

impl Serialize for Token {
    fn serialize<S: sorbit::ser_de::Serializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
        match self {
            Token::TinyAtom(value) => value.serialize(serializer),
            Token::ShortAtom(value) => value.serialize(serializer),
            Token::MediumAtom(value) => value.serialize(serializer),
            Token::LongAtom(value) => value.serialize(serializer),
            Token::StartList => (Tag::StartList as u8).serialize(serializer),
            Token::EndList => (Tag::EndList as u8).serialize(serializer),
            Token::StartName => (Tag::StartName as u8).serialize(serializer),
            Token::EndName => (Tag::EndName as u8).serialize(serializer),
            Token::Call => (Tag::Call as u8).serialize(serializer),
            Token::EndOfData => (Tag::EndOfData as u8).serialize(serializer),
            Token::EndOfSession => (Tag::EndOfSession as u8).serialize(serializer),
            Token::StartTransaction => (Tag::StartTransaction as u8).serialize(serializer),
            Token::EndTransaction => (Tag::EndTransaction as u8).serialize(serializer),
            Token::Empty => (Tag::Empty as u8).serialize(serializer),
        }
    }
}

impl Deserialize for Token {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
    where
        D: sorbit::ser_de::Deserializer,
    {
        let tag = u8::deserialize(deserializer)?;
        let token = match tag {
            x if x & 0b1000_0000 == Tag::TinyAtom as u8 => {
                Token::TinyAtom(TinyAtom::from_bytes(&[tag]).map_err(|_| D::Error::message("invalid tiny atom"))?)
            }
            x if x & 0b1100_0000 == Tag::ShortAtom as u8 => {
                let header =
                    ShortAtomHeader::from_bytes(&[tag]).map_err(|_| D::Error::message("invalid short atom"))?;
                let data = deserialize_items_by_len(deserializer, &header.length)?;
                Token::ShortAtom(ShortAtom {
                    tag: PhantomData,
                    byte: header.byte,
                    signed: header.signed,
                    length: PhantomData,
                    data,
                })
            }
            x if x & 0b1110_0000 == Tag::MediumAtom as u8 => {
                let bytes = [tag, u8::deserialize(deserializer)?];
                let header =
                    MediumAtomHeader::from_bytes(&bytes).map_err(|_| D::Error::message("invalid medium atom"))?;
                let data = deserialize_items_by_len(deserializer, &header.length)?;
                Token::MediumAtom(MediumAtom {
                    tag: PhantomData,
                    byte: header.byte,
                    signed: header.signed,
                    length: PhantomData,
                    data,
                })
            }
            x if x & 0b1111_1100 == Tag::LongAtom as u8 => {
                let bytes = [
                    tag,
                    u8::deserialize(deserializer)?,
                    u8::deserialize(deserializer)?,
                    u8::deserialize(deserializer)?,
                ];
                let header =
                    LongAtomHeader::from_bytes(&bytes).map_err(|_| D::Error::message("invalid medium atom"))?;
                let data = deserialize_items_by_len(deserializer, &header.length)?;
                Token::LongAtom(LongAtom {
                    tag: PhantomData,
                    byte: header.byte,
                    signed: header.signed,
                    length: PhantomData,
                    data,
                })
            }
            x if x == Tag::StartList as u8 => Token::StartList,
            x if x == Tag::EndList as u8 => Token::EndList,
            x if x == Tag::StartName as u8 => Token::StartName,
            x if x == Tag::EndName as u8 => Token::EndName,
            x if x == Tag::Call as u8 => Token::Call,
            x if x == Tag::EndOfData as u8 => Token::EndOfData,
            x if x == Tag::EndOfSession as u8 => Token::EndOfSession,
            x if x == Tag::StartTransaction as u8 => Token::StartTransaction,
            x if x == Tag::EndTransaction as u8 => Token::EndTransaction,
            x if x == Tag::Empty as u8 => Token::Empty,
            _ => return deserializer.error("invalid token tag"),
        };
        Ok(token)
    }
}

#[cfg(test)]
mod tests {
    use sorbit::ser_de::ToBytes;

    use super::*;

    #[rstest::rstest]
    #[case(Token::TinyAtom(TinyAtom { signed: false, data: 0x03 }), vec![0x03])]
    #[case(Token::TinyAtom(TinyAtom { signed: true, data: 0x03 }), vec![0x43])]
    #[case(Token::ShortAtom(ShortAtom { tag: PhantomData, byte: false, signed: true, length: PhantomData, data: vec![1, 2, 3] }), vec![0x93, 1, 2, 3])]
    #[case(Token::ShortAtom(ShortAtom { tag: PhantomData, byte: true, signed: false, length: PhantomData, data: vec![1, 2, 3] }), vec![0xA3, 1, 2, 3])]
    #[case(Token::MediumAtom(MediumAtom { tag: PhantomData, byte: false, signed: true, length: PhantomData, data: vec![1, 2, 3] }), vec![0b1100_1000, 3, 1, 2, 3])]
    #[case(Token::MediumAtom(MediumAtom { tag: PhantomData, byte: true, signed: false, length: PhantomData, data: vec![1, 2, 3] }), vec![0b1101_0000, 3, 1, 2, 3])]
    #[case(Token::LongAtom(LongAtom { tag: PhantomData, byte: false, signed: true, length: PhantomData, data: vec![1, 2, 3] }), vec![0b1110_0001, 0, 0, 3, 1, 2, 3])]
    #[case(Token::LongAtom(LongAtom { tag: PhantomData, byte: true, signed: false, length: PhantomData, data: vec![1, 2, 3] }), vec![0b1110_0010, 0, 0, 3, 1, 2, 3])]
    #[case(Token::StartList, vec![0xF0])]
    #[case(Token::EndList, vec![0xF1])]
    #[case(Token::StartName, vec![0xF2])]
    #[case(Token::EndName, vec![0xF3])]
    #[case(Token::Call, vec![0xF8])]
    #[case(Token::EndOfData, vec![0xF9])]
    #[case(Token::EndOfSession, vec![0xFA])]
    #[case(Token::StartTransaction, vec![0xFB])]
    #[case(Token::EndTransaction, vec![0xFC])]
    #[case(Token::Empty, vec![0xFF])]
    fn serialize(#[case] token: Token, #[case] bytes: Vec<u8>) {
        assert_eq!(token.to_bytes().as_ref(), Ok(&bytes));
        assert_eq!(Token::from_bytes(&bytes), Ok(token));
    }
}
