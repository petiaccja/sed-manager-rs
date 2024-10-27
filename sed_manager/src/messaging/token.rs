use crate::serialization::{Deserialize, Error, ItemRead, ItemWrite, Serialize, SerializeError};
use std::fmt::Display;

#[repr(u8)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Copy, Clone)]
pub enum Tag {
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

impl TryFrom<u8> for Tag {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            _ if value == Tag::TinyAtom as u8 => Ok(Tag::TinyAtom),
            _ if value == Tag::ShortAtom as u8 => Ok(Tag::ShortAtom),
            _ if value == Tag::MediumAtom as u8 => Ok(Tag::MediumAtom),
            _ if value == Tag::LongAtom as u8 => Ok(Tag::LongAtom),
            _ if value == Tag::StartList as u8 => Ok(Tag::StartList),
            _ if value == Tag::EndList as u8 => Ok(Tag::EndList),
            _ if value == Tag::StartName as u8 => Ok(Tag::StartName),
            _ if value == Tag::EndName as u8 => Ok(Tag::EndName),
            _ if value == Tag::Call as u8 => Ok(Tag::Call),
            _ if value == Tag::EndOfData as u8 => Ok(Tag::EndOfData),
            _ if value == Tag::EndOfSession as u8 => Ok(Tag::EndOfSession),
            _ if value == Tag::StartTransaction as u8 => Ok(Tag::StartTransaction),
            _ if value == Tag::EndTransaction as u8 => Ok(Tag::EndTransaction),
            _ if value == Tag::Empty as u8 => Ok(Tag::Empty),
            _ => Err(()),
        }
    }
}

#[repr(u8)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Copy, Clone)]
pub enum Mask {
    TinyAtom = 0b1000_0000,
    ShortAtom = 0b1100_0000,
    MediumAtom = 0b1110_0000,
    LongAtom = 0b1111_1000,
}

pub enum TokenizeError {
    EndOfStream,
    EndOfTokens,
    UnexpectedTag,
    UnexpectedSignedness,
    ExpectedInteger,
    ExpectedBytes,
    ContinuedBytesUnsupported,
    IntegerOverflow,
}

impl Error for TokenizeError {
    fn into_serialize_error(self) -> SerializeError {
        SerializeError::Other(Box::new(self))
    }
}

impl Display for TokenizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenizeError::EndOfStream => f.write_fmt(format_args!("end of stream")),
            TokenizeError::EndOfTokens => f.write_fmt(format_args!("end of tokens")),
            TokenizeError::UnexpectedTag => f.write_fmt(format_args!("unexpected tag")),
            TokenizeError::UnexpectedSignedness => f.write_fmt(format_args!("signedness does not match integer type")),
            TokenizeError::ExpectedInteger => f.write_fmt(format_args!("expected atom of type integer")),
            TokenizeError::ExpectedBytes => f.write_fmt(format_args!("expected atom of type bytes")),
            TokenizeError::ContinuedBytesUnsupported => {
                f.write_fmt(format_args!("continued bytes atoms are not supported"))
            }
            TokenizeError::IntegerOverflow => f.write_fmt(format_args!("integer atom too large for integer type")),
        }
    }
}

impl std::fmt::Debug for TokenizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (self as &dyn Display).fmt(f)
    }
}

pub fn get_tag(data_length: usize) -> Tag {
    if data_length <= 15 {
        Tag::ShortAtom
    } else if data_length <= 2047 {
        Tag::MediumAtom
    } else if data_length <= 16_777_215 {
        Tag::LongAtom
    } else {
        panic!("data length too large to fit into any atom")
    }
}

pub fn is_data(tag: Tag) -> bool {
    match tag {
        Tag::TinyAtom => true,
        Tag::ShortAtom => true,
        Tag::MediumAtom => true,
        Tag::LongAtom => true,
        _ => false,
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct Token {
    pub tag: Tag,
    pub is_byte: bool,
    pub is_signed: bool,
    pub data: Vec<u8>,
}

impl Default for Token {
    fn default() -> Self {
        Token { tag: Tag::Empty, is_byte: false, is_signed: false, data: vec![] }
    }
}

fn flag_bits(tag: Tag) -> (i32, i32) {
    // (byte/integer, sign/continued)
    match tag {
        Tag::TinyAtom => (-1, 6),
        Tag::ShortAtom => (5, 4),
        Tag::MediumAtom => (4, 3),
        Tag::LongAtom => (1, 0),
        _ => (-1, -1),
    }
}

impl Serialize<Token, u8> for Token {
    type Error = SerializeError;
    fn serialize(&self, stream: &mut crate::serialization::OutputStream<u8>) -> Result<(), Self::Error> {
        let (byte_bit, signed_bit) = flag_bits(self.tag);
        match self.tag {
            Tag::TinyAtom => {
                let header = (self.is_signed as u8) << signed_bit;
                let Some(data) = self.data.first() else {
                    return Err(SerializeError::InvalidRepr);
                };
                stream.write_one(header | (data & 0b0011_1111));
                Ok(())
            }
            Tag::ShortAtom => {
                let header =
                    (self.tag as u8) | ((self.is_byte as u8) << byte_bit) | ((self.is_signed as u8) << signed_bit);
                let len = (self.data.len() as u8) & 0b0000_1111;
                if len as usize != self.data.len() {
                    return Err(SerializeError::InvalidRepr);
                }
                stream.write_one(header | len);
                stream.write_exact(&self.data);
                Ok(())
            }
            Tag::MediumAtom => {
                let header =
                    (self.tag as u8) | ((self.is_byte as u8) << byte_bit) | ((self.is_signed as u8) << signed_bit);
                let len = (self.data.len() as u16) & 0b111_1111_1111;
                if len as usize != self.data.len() {
                    return Err(SerializeError::InvalidRepr);
                }
                stream.write_one(header | ((len >> 8) as u8));
                stream.write_one(len as u8);
                stream.write_exact(&self.data);
                Ok(())
            }
            Tag::LongAtom => {
                let header =
                    (self.tag as u8) | ((self.is_byte as u8) << byte_bit) | ((self.is_signed as u8) << signed_bit);
                let len = (self.data.len() as u32) & 0x00FF_FFFF;
                if len as usize != self.data.len() {
                    return Err(SerializeError::InvalidRepr);
                }
                stream.write_one(header | ((len >> 8) as u8));
                stream.write_exact(&len.to_be_bytes()[1..]);
                stream.write_exact(&self.data);
                Ok(())
            }
            tag => {
                stream.write_one(tag as u8);
                Ok(())
            }
        }
    }
}

fn extend_tiny_atom(data: u8, is_signed: bool) -> u8 {
    let extension = if is_signed {
        if (data >> 5) & 1 == 0 {
            0b0000_0000
        } else {
            0b1100_0000
        }
    } else {
        0b0000_0000
    };
    data | extension
}

impl Deserialize<Token, u8> for Token {
    type Error = SerializeError;
    fn deserialize(stream: &mut crate::serialization::InputStream<u8>) -> Result<Token, Self::Error> {
        let Some(&header) = stream.read_one() else {
            return Err(SerializeError::EndOfStream);
        };
        if header & (Mask::TinyAtom as u8) == Tag::TinyAtom as u8 {
            let (_, signed_bit) = flag_bits(Tag::TinyAtom);
            let is_signed = (header >> signed_bit) & 1 != 0;
            let data = extend_tiny_atom(header & 0b0011_1111, is_signed);
            Ok(Token { tag: Tag::TinyAtom, is_byte: false, is_signed: is_signed, data: vec![data] })
        } else if header & (Mask::ShortAtom as u8) == Tag::ShortAtom as u8 {
            let (byte_bit, signed_bit) = flag_bits(Tag::ShortAtom);
            let is_byte = (header >> byte_bit) & 1 != 0;
            let is_signed = (header >> signed_bit) & 1 != 0;
            let len = header & 0b1111;
            let Some(data) = stream.read_exact(len as usize) else {
                return Err(SerializeError::EndOfStream);
            };
            Ok(Token { tag: Tag::ShortAtom, is_byte: is_byte, is_signed: is_signed, data: data.into() })
        } else if header & (Mask::MediumAtom as u8) == Tag::MediumAtom as u8 {
            let (byte_bit, signed_bit) = flag_bits(Tag::MediumAtom);
            let is_byte = (header >> byte_bit) & 1 != 0;
            let is_signed = (header >> signed_bit) & 1 != 0;
            let len = if let Some(&len_lsb) = stream.read_one() {
                (((header & 0b111) as usize) << 8) | (len_lsb as usize)
            } else {
                return Err(SerializeError::EndOfStream);
            };
            let Some(data) = stream.read_exact(len) else {
                return Err(SerializeError::EndOfStream);
            };
            Ok(Token { tag: Tag::MediumAtom, is_byte: is_byte, is_signed: is_signed, data: data.into() })
        } else if header & (Mask::LongAtom as u8) == Tag::LongAtom as u8 {
            let (byte_bit, signed_bit) = flag_bits(Tag::LongAtom);
            let is_byte = (header >> byte_bit) & 1 != 0;
            let is_signed = (header >> signed_bit) & 1 != 0;
            let len = if let Some(len) = stream.read_exact(3) {
                ((len[0] as usize) << 16) | ((len[1] as usize) << 8) | (len[2] as usize)
            } else {
                return Err(SerializeError::EndOfStream);
            };
            let Some(data) = stream.read_exact(len) else {
                return Err(SerializeError::EndOfStream);
            };
            Ok(Token { tag: Tag::LongAtom, is_byte: is_byte, is_signed: is_signed, data: data.into() })
        } else {
            if let Ok(tag) = Tag::try_from(header) {
                Ok(Token { tag: tag, ..Default::default() })
            } else {
                Err(SerializeError::InvalidRepr)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::serialization::{InputStream, OutputStream};

    use super::*;

    #[test]
    fn serialize_tiny_atom() {
        let inputs = [
            Token { tag: Tag::TinyAtom, is_byte: false, is_signed: false, data: 53u8.to_be_bytes().into() },
            Token { tag: Tag::TinyAtom, is_byte: false, is_signed: true, data: (-27i8).to_be_bytes().into() },
        ];
        for input in inputs {
            let mut os = OutputStream::<u8>::new();
            input.serialize(&mut os).unwrap();
            let mut is = InputStream::<u8>::from(os.take());
            let output = Token::deserialize(&mut is).unwrap();
            assert_eq!(input, output);
        }
    }

    #[test]
    fn serialize_short_atom() {
        let inputs = [
            Token { tag: Tag::ShortAtom, is_byte: false, is_signed: false, data: 53u8.to_be_bytes().into() },
            Token { tag: Tag::ShortAtom, is_byte: false, is_signed: true, data: (-27i8).to_be_bytes().into() },
            Token { tag: Tag::ShortAtom, is_byte: true, is_signed: false, data: vec![0x65; 15] },
        ];
        for input in inputs {
            let mut os = OutputStream::<u8>::new();
            input.serialize(&mut os).unwrap();
            let mut is = InputStream::<u8>::from(os.take());
            let output = Token::deserialize(&mut is).unwrap();
            assert_eq!(input, output);
        }
    }

    #[test]
    fn serialize_medium_atom() {
        let inputs = [
            Token { tag: Tag::MediumAtom, is_byte: false, is_signed: false, data: 53u8.to_be_bytes().into() },
            Token { tag: Tag::MediumAtom, is_byte: false, is_signed: true, data: (-27i8).to_be_bytes().into() },
            Token { tag: Tag::MediumAtom, is_byte: true, is_signed: false, data: vec![0x65; 15] },
        ];
        for input in inputs {
            let mut os = OutputStream::<u8>::new();
            input.serialize(&mut os).unwrap();
            let mut is = InputStream::<u8>::from(os.take());
            let output = Token::deserialize(&mut is).unwrap();
            assert_eq!(input, output);
        }
    }

    #[test]
    fn serialize_long_atom() {
        let inputs = [
            Token { tag: Tag::LongAtom, is_byte: false, is_signed: false, data: 53u8.to_be_bytes().into() },
            Token { tag: Tag::LongAtom, is_byte: false, is_signed: true, data: (-27i8).to_be_bytes().into() },
            Token { tag: Tag::LongAtom, is_byte: true, is_signed: false, data: vec![0x65; 15] },
        ];
        for input in inputs {
            let mut os = OutputStream::<u8>::new();
            input.serialize(&mut os).unwrap();
            let mut is = InputStream::<u8>::from(os.take());
            let output = Token::deserialize(&mut is).unwrap();
            assert_eq!(input, output);
        }
    }
}
