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

#[derive(Debug)]
pub enum TokenParsingError {
    EndOfStream,
    EndOfData,
    IncorrectTag,
    IncorrectSignedness,
    IntegerExpected,
    BytesExpected,
    SignedExpected,
    NonContinuedExpected,
    IntegerOverflow,
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
        Token {
            tag: Tag::Empty,
            is_byte: false,
            is_signed: false,
            data: vec![],
        }
    }
}

pub trait Tokenize<T> {
    fn tokenize(&self) -> Vec<Token>;
}

pub trait FromTokens<T> {
    fn from_tokens(tokens: &[Token]) -> Result<(T, &[Token]), TokenParsingError>;
}
