use super::token::{get_tag, is_data, Tag, Token, TokenParsingError};
use super::value::{Bytes, Command, List, Named, Storage, Value};
use crate::serialization::{Deserialize, InputStream, ItemRead, ItemWrite, OutputStream, Serialize};

macro_rules! impl_tokenize_integer {
    ($int_ty:ty, $signed:expr) => {
        impl Serialize<$int_ty, Token> for $int_ty {
            type Error = TokenParsingError;
            fn serialize(&self, stream: &mut OutputStream<Token>) -> Result<(), Self::Error> {
                let token = Token {
                    tag: get_tag(size_of_val(self)),
                    is_byte: false,
                    is_signed: $signed,
                    data: self.to_be_bytes().as_ref().into(),
                };
                stream.write_one(token);
                Ok(())
            }
        }
    };
}

macro_rules! impl_from_tokens_integer {
    ($int_ty:ty, $signed:expr) => {
        impl Deserialize<$int_ty, Token> for $int_ty {
            type Error = TokenParsingError;
            fn deserialize(stream: &mut InputStream<Token>) -> Result<$int_ty, Self::Error> {
                if let Some(token) = stream.read_one() {
                    if !is_data(token.tag) {
                        Err(TokenParsingError::IncorrectTag)
                    } else if token.is_byte {
                        Err(TokenParsingError::IntegerExpected)
                    } else if token.is_signed != $signed {
                        Err(TokenParsingError::IncorrectSignedness)
                    } else if token.data.len() > size_of::<$int_ty>() {
                        Err(TokenParsingError::IntegerOverflow)
                    } else {
                        let leading_byte = token.data.first().unwrap_or(&0u8);
                        let twos_complement_fill = if 0u8 != leading_byte & 0b1000_0000u8 { 0xFFu8 } else { 0x00u8 };
                        let unsigned_fill = 0u8;
                        let fill = if token.is_signed { twos_complement_fill } else { unsigned_fill };
                        let mut bytes = [fill; size_of::<$int_ty>()];
                        for i in 0..std::cmp::min(bytes.len(), token.data.len()) {
                            bytes[i] = token.data[token.data.len() - i - 1];
                        }
                        Ok(<$int_ty>::from_le_bytes(bytes))
                    }
                } else {
                    Err(TokenParsingError::EndOfStream)
                }
            }
        }
    };
}

impl_tokenize_integer!(i8, true);
impl_tokenize_integer!(i16, true);
impl_tokenize_integer!(i32, true);
impl_tokenize_integer!(i64, true);
impl_tokenize_integer!(u8, false);
impl_tokenize_integer!(u16, false);
impl_tokenize_integer!(u32, false);
impl_tokenize_integer!(u64, false);

impl_from_tokens_integer!(i8, true);
impl_from_tokens_integer!(i16, true);
impl_from_tokens_integer!(i32, true);
impl_from_tokens_integer!(i64, true);
impl_from_tokens_integer!(u8, false);
impl_from_tokens_integer!(u16, false);
impl_from_tokens_integer!(u32, false);
impl_from_tokens_integer!(u64, false);

impl Serialize<Command, Token> for Command {
    type Error = TokenParsingError;
    fn serialize(&self, stream: &mut OutputStream<Token>) -> Result<(), Self::Error> {
        let tag = match self {
            Command::Call => Tag::Call,
            Command::EndOfData => Tag::EndOfData,
            Command::EndOfSession => Tag::EndOfSession,
            Command::StartTransaction => Tag::StartTransaction,
            Command::EndTransaction => Tag::EndTransaction,
            Command::Empty => Tag::Empty,
        };
        stream.write_one(Token { tag: tag, ..Default::default() });
        Ok(())
    }
}

impl Deserialize<Command, Token> for Command {
    type Error = TokenParsingError;
    fn deserialize(stream: &mut InputStream<Token>) -> Result<Command, Self::Error> {
        if let Some(token) = stream.read_one() {
            let command = match token.tag {
                Tag::Call => Some(Command::Call),
                Tag::EndOfData => Some(Command::EndOfData),
                Tag::EndOfSession => Some(Command::EndOfSession),
                Tag::StartTransaction => Some(Command::StartTransaction),
                Tag::EndTransaction => Some(Command::EndTransaction),
                Tag::Empty => Some(Command::Empty),
                _ => None,
            };
            match command {
                Some(c) => Ok(c),
                None => Err(TokenParsingError::IncorrectTag),
            }
        } else {
            Err(TokenParsingError::EndOfStream)
        }
    }
}

impl Serialize<Named, Token> for Named {
    type Error = TokenParsingError;
    fn serialize(&self, stream: &mut OutputStream<Token>) -> Result<(), Self::Error> {
        let start_name = Token { tag: Tag::StartName, ..Default::default() };
        let end_name = Token { tag: Tag::EndName, ..Default::default() };

        stream.write_one(start_name);
        self.name.serialize(stream)?;
        self.value.serialize(stream)?;
        stream.write_one(end_name);
        Ok(())
    }
}

impl Deserialize<Named, Token> for Named {
    type Error = TokenParsingError;
    fn deserialize(stream: &mut InputStream<Token>) -> Result<Named, Self::Error> {
        fn is_terminator(maybe_token: Option<&Token>) -> bool {
            match maybe_token {
                Some(token) => token.tag == Tag::EndName,
                None => false,
            }
        }

        if let Some(token) = stream.read_one() {
            if token.tag != Tag::StartName {
                Err(TokenParsingError::IncorrectTag)
            } else {
                let mut named = Named { name: Value::empty(), value: Value::empty() };

                if !is_terminator(stream.peek_one()) {
                    named.name = Value::deserialize(stream)?;
                }

                if !is_terminator(stream.peek_one()) {
                    named.value = Value::deserialize(stream)?;
                }

                if is_terminator(stream.read_one()) {
                    Ok(named)
                } else {
                    Err(TokenParsingError::IncorrectTag)
                }
            }
        } else {
            Err(TokenParsingError::EndOfStream)
        }
    }
}

impl Serialize<Bytes, Token> for Bytes {
    type Error = TokenParsingError;
    fn serialize(&self, stream: &mut OutputStream<Token>) -> Result<(), Self::Error> {
        let token = Token { tag: get_tag(self.len()), is_byte: true, is_signed: false, data: self.clone() };
        stream.write_one(token);
        Ok(())
    }
}

impl Deserialize<Bytes, Token> for Bytes {
    type Error = TokenParsingError;
    fn deserialize(stream: &mut InputStream<Token>) -> Result<Bytes, Self::Error> {
        if let Some(token) = stream.read_one() {
            if !is_data(token.tag) {
                Err(TokenParsingError::IncorrectTag)
            } else if !token.is_byte {
                Err(TokenParsingError::BytesExpected)
            } else if token.is_signed != false {
                Err(TokenParsingError::NonContinuedExpected)
            } else {
                Ok(token.data.clone())
            }
        } else {
            Err(TokenParsingError::EndOfStream)
        }
    }
}

impl Serialize<List, Token> for List {
    type Error = TokenParsingError;
    fn serialize(&self, stream: &mut OutputStream<Token>) -> Result<(), Self::Error> {
        let start_list = Token { tag: Tag::StartList, ..Default::default() };
        let end_list = Token { tag: Tag::EndList, ..Default::default() };

        stream.write_one(start_list);
        for item in self {
            item.serialize(stream)?;
        }
        stream.write_one(end_list);

        Ok(())
    }
}

impl Deserialize<List, Token> for List {
    type Error = TokenParsingError;
    fn deserialize(stream: &mut InputStream<Token>) -> Result<List, Self::Error> {
        fn is_terminator(token: Option<&Token>) -> bool {
            match token {
                Some(token) => token.tag == Tag::EndList,
                None => false,
            }
        }

        if let Some(token) = stream.read_one() {
            if token.tag != Tag::StartList {
                Err(TokenParsingError::IncorrectTag)
            } else {
                let mut list = List::new();

                while !is_terminator(stream.peek_one()) {
                    list.push(Value::deserialize(stream)?);
                }

                assert!(is_terminator(stream.read_one()));
                Ok(list)
            }
        } else {
            Err(TokenParsingError::EndOfStream)
        }
    }
}

impl Serialize<Value, Token> for Value {
    type Error = TokenParsingError;
    fn serialize(&self, stream: &mut OutputStream<Token>) -> Result<(), Self::Error> {
        match self.storage() {
            Storage::Empty => Ok(()),
            Storage::Int8(value) => value.serialize(stream),
            Storage::Int16(value) => value.serialize(stream),
            Storage::Int32(value) => value.serialize(stream),
            Storage::Int64(value) => value.serialize(stream),
            Storage::Uint8(value) => value.serialize(stream),
            Storage::Uint16(value) => value.serialize(stream),
            Storage::Uint32(value) => value.serialize(stream),
            Storage::Uint64(value) => value.serialize(stream),
            Storage::Command(value) => value.serialize(stream),
            Storage::Named(value) => value.serialize(stream),
            Storage::Bytes(value) => value.serialize(stream),
            Storage::List(value) => value.serialize(stream),
        }
    }
}

impl Deserialize<Value, Token> for Value {
    type Error = TokenParsingError;
    fn deserialize(stream: &mut InputStream<Token>) -> Result<Value, Self::Error> {
        fn parse_value(stream: &mut InputStream<Token>) -> Result<Value, TokenParsingError> {
            let token = stream.peek_one().unwrap();
            assert!(is_data(token.tag));
            if token.is_byte {
                Ok(Value::from(Bytes::deserialize(stream)?))
            } else {
                if token.is_signed {
                    match token.data.len() {
                        0..=1 => Ok(Value::from(i8::deserialize(stream)?)),
                        2..=2 => Ok(Value::from(i16::deserialize(stream)?)),
                        3..=4 => Ok(Value::from(i32::deserialize(stream)?)),
                        5..=8 => Ok(Value::from(i64::deserialize(stream)?)),
                        _ => Err(TokenParsingError::IntegerOverflow),
                    }
                } else {
                    match token.data.len() {
                        0..=1 => Ok(Value::from(u8::deserialize(stream)?)),
                        2..=2 => Ok(Value::from(u16::deserialize(stream)?)),
                        3..=4 => Ok(Value::from(u32::deserialize(stream)?)),
                        5..=8 => Ok(Value::from(u64::deserialize(stream)?)),
                        _ => Err(TokenParsingError::IntegerOverflow),
                    }
                }
            }
        }

        if let Some(peek) = stream.peek_one() {
            match peek.tag {
                Tag::TinyAtom => parse_value(stream),
                Tag::ShortAtom => parse_value(stream),
                Tag::MediumAtom => parse_value(stream),
                Tag::LongAtom => parse_value(stream),
                Tag::StartList => Ok(Value::from(List::deserialize(stream)?)),
                Tag::EndList => Err(TokenParsingError::IncorrectTag),
                Tag::StartName => Ok(Value::from(Named::deserialize(stream)?)),
                Tag::EndName => Err(TokenParsingError::IncorrectTag),
                Tag::Call => Ok(Value::from(Command::deserialize(stream)?)),
                Tag::EndOfData => Ok(Value::from(Command::deserialize(stream)?)),
                Tag::EndOfSession => Ok(Value::from(Command::deserialize(stream)?)),
                Tag::StartTransaction => Ok(Value::from(Command::deserialize(stream)?)),
                Tag::EndTransaction => Ok(Value::from(Command::deserialize(stream)?)),
                Tag::Empty => Ok(Value::from(Command::deserialize(stream)?)),
            }
        } else {
            Err(TokenParsingError::EndOfStream)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_tokenize_integer {
        ($int_ty:ty, $name:ident) => {
            #[test]
            fn $name() {
                for input in [<$int_ty>::min_value(), <$int_ty>::max_value()] {
                    let mut os = OutputStream::<Token>::new();
                    assert!(input.serialize(&mut os).is_ok());
                    let mut is = InputStream::<Token>::from(os.take());
                    match <$int_ty>::deserialize(&mut is) {
                        Ok(value) => assert_eq!(value, input),
                        Err(err) => assert!(false, "{:?}", err),
                    };
                }
            }
        };
    }

    test_tokenize_integer!(i8, tokenize_i8);
    test_tokenize_integer!(i16, tokenize_i16);
    test_tokenize_integer!(i32, tokenize_i32);
    test_tokenize_integer!(i64, tokenize_i64);
    test_tokenize_integer!(u8, tokenize_u8);
    test_tokenize_integer!(u16, tokenize_u16);
    test_tokenize_integer!(u32, tokenize_u32);
    test_tokenize_integer!(u64, tokenize_u64);

    #[test]
    fn from_tokens_extension_positive() {
        let input = 127_u8;
        let tokens = vec![Token { tag: Tag::ShortAtom, is_byte: false, is_signed: true, data: vec![input] }];
        let mut is = InputStream::<Token>::from(tokens);
        match i64::deserialize(&mut is) {
            Ok(value) => assert_eq!(value, input as i64),
            Err(err) => assert!(false, "{:?}", err),
        };
    }

    #[test]
    fn from_tokens_extension_negative() {
        let input = -128_i8;
        let tokens = vec![Token {
            tag: Tag::ShortAtom,
            is_byte: false,
            is_signed: true,
            data: vec![unsafe { std::mem::transmute::<i8, u8>(input) }],
        }];
        let mut is = InputStream::<Token>::from(tokens);
        match i64::deserialize(&mut is) {
            Ok(value) => assert_eq!(value, input as i64),
            Err(err) => assert!(false, "{:?}", err),
        };
    }

    #[test]
    fn from_tokens_extension_unsigned() {
        let input = 255_u8;
        let tokens = vec![Token { tag: Tag::ShortAtom, is_byte: false, is_signed: false, data: vec![input] }];
        let mut is = InputStream::<Token>::from(tokens);
        match u64::deserialize(&mut is) {
            Ok(value) => assert_eq!(value, input as u64),
            Err(err) => assert!(false, "{:?}", err),
        };
    }

    #[test]
    fn tokenize_named() {
        let inputs = vec![
            Named { name: Value::empty(), value: Value::empty() },
            Named { name: Value::from(234_u32), value: Value::empty() },
            Named { name: Value::from(234_u32), value: Value::from(5474_u32) },
            Named {
                name: Value::from(234_u32),
                value: Value::from(Named { name: Value::from(2893_u32), value: Value::from(9634_u32) }),
            },
        ];
        for input in inputs {
            let mut os = OutputStream::<Token>::new();
            assert!(input.serialize(&mut os).is_ok());
            let mut is = InputStream::<Token>::from(os.take());
            match Named::deserialize(&mut is) {
                Ok(value) => assert_eq!(value, input),
                Err(err) => assert!(false, "{:?}", err),
            };
        }
    }

    #[test]
    fn tokenize_bytes() {
        let input = vec![0xAD_u8, 0xEF_u8];
        let mut os = OutputStream::<Token>::new();
        assert!(input.serialize(&mut os).is_ok());
        let mut is = InputStream::<Token>::from(os.take());
        match Bytes::deserialize(&mut is) {
            Ok(value) => assert_eq!(value, input),
            Err(err) => assert!(false, "{:?}", err),
        };
    }

    #[test]
    fn tokenize_list() {
        let input = vec![
            Value::from(27345_u16),
            Value::from(vec![Value::from(2365_i32), Value::from(62735345_i64)]),
        ];
        let mut os = OutputStream::<Token>::new();
        assert!(input.serialize(&mut os).is_ok());
        let mut is = InputStream::<Token>::from(os.take());
        match List::deserialize(&mut is) {
            Ok(value) => assert_eq!(value, input),
            Err(err) => assert!(false, "{:?}", err),
        };
    }

    #[test]
    fn tokenize_value() {
        let inputs = vec![
            Value::from(12_i8),
            Value::from(12_i16),
            Value::from(12_i32),
            Value::from(12_i64),
            Value::from(12_u8),
            Value::from(12_u16),
            Value::from(12_u32),
            Value::from(12_u64),
            Value::from(Command::Call),
            Value::from(Command::EndOfData),
            Value::from(Command::EndOfSession),
            Value::from(Command::StartTransaction),
            Value::from(Command::EndTransaction),
            Value::from(Command::Empty),
            Value::from(Named { name: Value::from(1_i32), value: Value::from(7_i32) }),
            Value::from(vec![1_u8, 2_u8]),
            Value::from(vec![Value::from(1_i32), Value::from(7_i32)]),
        ];
        for input in inputs {
            let mut os = OutputStream::<Token>::new();
            assert!(input.serialize(&mut os).is_ok());
            let mut is = InputStream::<Token>::from(os.take());
            match Value::deserialize(&mut is) {
                Ok(value) => assert_eq!(value, input),
                Err(err) => assert!(false, "{:?}", err),
            };
        }
    }
}
