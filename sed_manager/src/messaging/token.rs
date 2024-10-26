use crate::serialization::{Error, SerializeError};
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
