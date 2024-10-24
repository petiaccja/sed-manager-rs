use super::token::{get_tag, is_data, FromTokens, Tag, Token, TokenParsingError, Tokenize};
use super::value::{Bytes, Command, List, Named, Storage, Value};
use crate::serialization::serializer::{Serialize, Deserialize};

macro_rules! impl_tokenize_integer {
    ($int_ty:ty, $signed:expr) => {
        impl Tokenize<$int_ty> for $int_ty {
            fn tokenize(&self) -> Vec<Token> {
                let token = Token {
                    tag: get_tag(size_of_val(self)),
                    is_byte: false,
                    is_signed: $signed,
                    data: self.to_be_bytes().as_ref().into(),
                };
                vec![token]
            }
        }
    };
}

macro_rules! impl_from_tokens_integer {
    ($int_ty:ty, $signed:expr) => {
        impl FromTokens<$int_ty> for $int_ty {
            fn from_tokens(tokens: &[Token]) -> Result<($int_ty, &[Token]), super::token::TokenParsingError> {
                if let Some(token) = tokens.first() {
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
                        Ok((<$int_ty>::from_le_bytes(bytes), &tokens[1..]))
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

impl Tokenize<Command> for Command {
    fn tokenize(&self) -> Vec<Token> {
        let tag = match self {
            Command::Call => Tag::Call,
            Command::EndOfData => Tag::EndOfData,
            Command::EndOfSession => Tag::EndOfSession,
            Command::StartTransaction => Tag::StartTransaction,
            Command::EndTransaction => Tag::EndTransaction,
            Command::Empty => Tag::Empty,
        };
        vec![Token { tag: tag, ..Default::default() }]
    }
}

impl FromTokens<Command> for Command {
    fn from_tokens(tokens: &[Token]) -> Result<(Command, &[Token]), TokenParsingError> {
        if let Some(token) = tokens.first() {
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
                Some(c) => Ok((c, &tokens[1..])),
                None => Err(TokenParsingError::IncorrectTag),
            }
        } else {
            Err(TokenParsingError::EndOfStream)
        }
    }
}

impl Tokenize<Named> for Named {
    fn tokenize(&self) -> Vec<Token> {
        let mut tokens = Vec::<Token>::new();

        let start_name = Token { tag: Tag::StartName, ..Default::default() };
        let end_name = Token { tag: Tag::EndName, ..Default::default() };

        tokens.push(start_name);
        tokens.append(&mut self.name.tokenize());
        tokens.append(&mut self.value.tokenize());
        tokens.push(end_name);
        tokens
    }
}

impl FromTokens<Named> for Named {
    fn from_tokens(tokens: &[Token]) -> Result<(Named, &[Token]), TokenParsingError> {
        fn is_terminator(tokens: &[Token]) -> bool {
            let first = tokens.first();
            match first {
                Some(token) => token.tag == Tag::EndName,
                None => false,
            }
        }

        if let Some(token) = tokens.first() {
            if token.tag != Tag::StartName {
                Err(TokenParsingError::IncorrectTag)
            } else {
                let mut named = Named { name: Value::empty(), value: Value::empty() };
                let mut rest = &tokens[1..];

                if !is_terminator(rest) {
                    match Value::from_tokens(rest) {
                        Ok((name, new_rest)) => {
                            named.name = name;
                            rest = new_rest;
                        }
                        Err(err) => {
                            return Err(err);
                        }
                    }
                }

                if !is_terminator(rest) {
                    match Value::from_tokens(rest) {
                        Ok((value, new_rest)) => {
                            named.value = value;
                            rest = new_rest;
                        }
                        Err(err) => {
                            return Err(err);
                        }
                    }
                }

                if is_terminator(rest) {
                    Ok((named, &rest[1..]))
                } else {
                    Err(TokenParsingError::IncorrectTag)
                }
            }
        } else {
            Err(TokenParsingError::EndOfStream)
        }
    }
}

impl Tokenize<Bytes> for Bytes {
    fn tokenize(&self) -> Vec<Token> {
        let token = Token { tag: get_tag(self.len()), is_byte: true, is_signed: false, data: self.clone() };
        vec![token]
    }
}

impl FromTokens<Bytes> for Bytes {
    fn from_tokens(tokens: &[Token]) -> Result<(Bytes, &[Token]), TokenParsingError> {
        if let Some(token) = tokens.first() {
            if !is_data(token.tag) {
                Err(TokenParsingError::IncorrectTag)
            } else if !token.is_byte {
                Err(TokenParsingError::BytesExpected)
            } else if token.is_signed != false {
                Err(TokenParsingError::NonContinuedExpected)
            } else {
                Ok((token.data.clone(), &tokens[1..]))
            }
        } else {
            Err(TokenParsingError::EndOfStream)
        }
    }
}

impl Tokenize<List> for List {
    fn tokenize(&self) -> Vec<Token> {
        let mut tokens = Vec::<Token>::new();

        let start_list = Token { tag: Tag::StartList, ..Default::default() };
        let end_list = Token { tag: Tag::EndList, ..Default::default() };

        tokens.push(start_list);
        for item in self {
            tokens.append(&mut item.tokenize());
        }
        tokens.push(end_list);
        tokens
    }
}

impl FromTokens<List> for List {
    fn from_tokens(tokens: &[Token]) -> Result<(List, &[Token]), TokenParsingError> {
        fn is_terminator(tokens: &[Token]) -> bool {
            let first = tokens.first();
            match first {
                Some(token) => token.tag == Tag::EndList,
                None => false,
            }
        }

        if let Some(token) = tokens.first() {
            if token.tag != Tag::StartList {
                Err(TokenParsingError::IncorrectTag)
            } else {
                let mut list = List::new();
                let mut rest = &tokens[1..];

                while !is_terminator(rest) {
                    let maybe_value = Value::from_tokens(rest);
                    match maybe_value {
                        Ok((value, new_rest)) => {
                            list.push(value);
                            rest = new_rest;
                        }
                        Err(err) => {
                            return Err(err);
                        }
                    }
                }
                Ok((list, &rest[1..]))
            }
        } else {
            Err(TokenParsingError::EndOfStream)
        }
    }
}

impl Tokenize<Value> for Value {
    fn tokenize(&self) -> Vec<Token> {
        match self.storage() {
            Storage::Empty => vec![],
            Storage::Int8(value) => value.tokenize(),
            Storage::Int16(value) => value.tokenize(),
            Storage::Int32(value) => value.tokenize(),
            Storage::Int64(value) => value.tokenize(),
            Storage::Uint8(value) => value.tokenize(),
            Storage::Uint16(value) => value.tokenize(),
            Storage::Uint32(value) => value.tokenize(),
            Storage::Uint64(value) => value.tokenize(),
            Storage::Command(value) => value.tokenize(),
            Storage::Named(value) => value.tokenize(),
            Storage::Bytes(value) => value.tokenize(),
            Storage::List(value) => value.tokenize(),
        }
    }
}

impl FromTokens<Value> for Value {
    fn from_tokens(tokens: &[Token]) -> Result<(Value, &[Token]), TokenParsingError> {
        fn to_value<T>(
            content: Result<(T, &[Token]), TokenParsingError>,
        ) -> Result<(Value, &[Token]), TokenParsingError>
        where
            Value: From<T>,
        {
            match content {
                Ok((content_, rest)) => Ok((Value::from(content_), rest)),
                Err(err) => Err(err),
            }
        }

        fn parse_value(tokens: &[Token]) -> Result<(Value, &[Token]), TokenParsingError> {
            let token = tokens.first().unwrap();
            assert!(is_data(token.tag));
            if token.is_byte {
                to_value(Bytes::from_tokens(&tokens))
            } else {
                if token.is_signed {
                    match token.data.len() {
                        0..=1 => to_value(i8::from_tokens(tokens)),
                        2..=2 => to_value(i16::from_tokens(tokens)),
                        3..=4 => to_value(i32::from_tokens(tokens)),
                        5..=8 => to_value(i64::from_tokens(tokens)),
                        _ => Err(TokenParsingError::IntegerOverflow),
                    }
                } else {
                    match token.data.len() {
                        0..=1 => to_value(u8::from_tokens(tokens)),
                        2..=2 => to_value(u16::from_tokens(tokens)),
                        3..=4 => to_value(u32::from_tokens(tokens)),
                        5..=8 => to_value(u64::from_tokens(tokens)),
                        _ => Err(TokenParsingError::IntegerOverflow),
                    }
                }
            }
        }

        if let Some(peek) = tokens.first() {
            match peek.tag {
                Tag::TinyAtom => parse_value(tokens),
                Tag::ShortAtom => parse_value(tokens),
                Tag::MediumAtom => parse_value(tokens),
                Tag::LongAtom => parse_value(tokens),
                Tag::StartList => to_value(List::from_tokens(tokens)),
                Tag::EndList => Err(TokenParsingError::IncorrectTag),
                Tag::StartName => to_value(Named::from_tokens(tokens)),
                Tag::EndName => Err(TokenParsingError::IncorrectTag),
                Tag::Call => to_value(Command::from_tokens(tokens)),
                Tag::EndOfData => to_value(Command::from_tokens(tokens)),
                Tag::EndOfSession => to_value(Command::from_tokens(tokens)),
                Tag::StartTransaction => to_value(Command::from_tokens(tokens)),
                Tag::EndTransaction => to_value(Command::from_tokens(tokens)),
                Tag::Empty => to_value(Command::from_tokens(tokens)),
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
                    let token = input.tokenize();
                    let content = <$int_ty>::from_tokens(&token);
                    assert!(content.is_ok());
                    assert_eq!(content.unwrap().0, input);
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
        let value = 127_u8;
        let tokens = [Token { tag: Tag::ShortAtom, is_byte: false, is_signed: true, data: vec![value] }];
        let content = i64::from_tokens(&tokens);
        assert!(content.is_ok());
        assert_eq!(content.unwrap().0, i64::from(value));
    }

    #[test]
    fn from_tokens_extension_negative() {
        let value = -128_i8;
        let tokens = [Token {
            tag: Tag::ShortAtom,
            is_byte: false,
            is_signed: true,
            data: vec![unsafe { std::mem::transmute::<i8, u8>(value) }],
        }];
        let content = i64::from_tokens(&tokens);
        assert!(content.is_ok());
        assert_eq!(content.unwrap().0, i64::from(value));
    }

    #[test]
    fn from_tokens_extension_unsigned() {
        let value = 255_u8;
        let tokens = [Token { tag: Tag::ShortAtom, is_byte: false, is_signed: false, data: vec![value] }];
        let content = u64::from_tokens(&tokens);
        assert!(content.is_ok());
        assert_eq!(content.unwrap().0, u64::from(value));
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
            let token = input.tokenize();
            let content = Named::from_tokens(&token);
            assert!(content.is_ok());
            assert_eq!(content.unwrap().0, input);
        }
    }

    #[test]
    fn tokenize_bytes() {
        let input = vec![0xAD_u8, 0xEF_u8];
        let token = input.tokenize();
        let content = Bytes::from_tokens(&token);
        assert!(content.is_ok());
        assert_eq!(content.unwrap().0, input);
    }

    #[test]
    fn tokenize_list() {
        let input = vec![
            Value::from(27345_u16),
            Value::from(vec![Value::from(2365_i32), Value::from(62735345_i64)]),
        ];
        let token = input.tokenize();
        let content = List::from_tokens(&token);
        assert!(content.is_ok());
        assert_eq!(content.unwrap().0, input);
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
            let token = input.tokenize();
            let content = Value::from_tokens(&token);
            assert!(content.is_ok());
            assert_eq!(content.unwrap().0, input);
        }
    }
}
