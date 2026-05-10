use sorbit::ser_de::{Deserialize as _, Deserializer, Serialize as _, Serializer};

use crate::token::{Detokenizer, Tokenize, Tokenizer};

use super::command::Command;
use super::error::Error;
use super::token::Token;
use super::tokenize::TokenType;

macro_rules! convert {
    ($token:expr, $type:ty) => {{
        let token = $token;
        let ty = TokenType::from(&token);
        <$type>::try_from(token).map_err(|_| Error::CanNotConvert { from: ty.into(), to: stringify!($type) })
    }};
}

pub struct SorbitTokenizer<S>
where
    S: Serializer,
    Error: From<<S as Serializer>::Error>,
{
    serializer: S,
}

impl<S> SorbitTokenizer<S>
where
    S: Serializer,
    Error: From<<S as Serializer>::Error>,
{
    pub fn new(serializer: S) -> Self {
        Self { serializer }
    }

    pub fn take(self) -> S {
        self.serializer
    }
}

impl<S> Tokenizer for SorbitTokenizer<S>
where
    S: Serializer,
    Error: From<<S as Serializer>::Error>,
{
    type Error = Error;

    fn tokenize_i8(&mut self, value: i8) -> Result<(), Error> {
        Token::from(value).serialize(&mut self.serializer)?;
        Ok(())
    }

    fn tokenize_i16(&mut self, value: i16) -> Result<(), Error> {
        Token::from(value).serialize(&mut self.serializer)?;
        Ok(())
    }

    fn tokenize_i32(&mut self, value: i32) -> Result<(), Error> {
        Token::from(value).serialize(&mut self.serializer)?;
        Ok(())
    }

    fn tokenize_i64(&mut self, value: i64) -> Result<(), Error> {
        Token::from(value).serialize(&mut self.serializer)?;
        Ok(())
    }

    fn tokenize_u8(&mut self, value: u8) -> Result<(), Error> {
        Token::from(value).serialize(&mut self.serializer)?;
        Ok(())
    }

    fn tokenize_u16(&mut self, value: u16) -> Result<(), Error> {
        Token::from(value).serialize(&mut self.serializer)?;
        Ok(())
    }

    fn tokenize_u32(&mut self, value: u32) -> Result<(), Error> {
        Token::from(value).serialize(&mut self.serializer)?;
        Ok(())
    }

    fn tokenize_u64(&mut self, value: u64) -> Result<(), Error> {
        Token::from(value).serialize(&mut self.serializer)?;
        Ok(())
    }

    fn tokenize_command(&mut self, value: Command) -> Result<(), Error> {
        Token::from(value).serialize(&mut self.serializer)?;
        Ok(())
    }

    fn tokenize_named(&mut self, name: impl Tokenize, value: impl Tokenize) -> Result<(), Error> {
        Token::StartName.serialize(&mut self.serializer)?;
        name.tokenize(self)?;
        value.tokenize(self)?;
        Token::EndName.serialize(&mut self.serializer)?;
        Ok(())
    }

    fn tokenize_list(&mut self, items: impl FnOnce(&mut Self) -> Result<(), Error>) -> Result<(), Error> {
        Token::StartList.serialize(&mut self.serializer)?;
        items(self)?;
        Token::EndList.serialize(&mut self.serializer)?;
        Ok(())
    }

    fn tokenize_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
        Token::try_from(bytes)
            .map_err(|_| Error::PayloadTooBig { len: bytes.len() })?
            .serialize(&mut self.serializer)?;
        Ok(())
    }
}

pub struct SorbitDetokenizer<D>
where
    D: Deserializer,
    Error: From<<D as Deserializer>::Error>,
{
    deserializer: D,
    next_token: Option<Token>,
}

impl<D> SorbitDetokenizer<D>
where
    D: Deserializer,
    Error: From<<D as Deserializer>::Error>,
{
    pub fn new(deserializer: D) -> Self {
        Self { deserializer, next_token: None }
    }

    pub fn take(self) -> D {
        self.deserializer
    }

    fn read_token(&mut self) -> Result<Token, Error> {
        match self.next_token.take() {
            Some(token) => Ok(token),
            None => Token::deserialize(&mut self.deserializer).map_err(|e| e.into()),
        }
    }

    fn peek_token(&mut self) -> Result<&Token, Error> {
        match &mut self.next_token {
            Some(token) => Ok(token),
            next_token @ None => {
                next_token.replace(Token::deserialize(&mut self.deserializer)?);
                Ok(next_token.as_ref().expect("token has just been written"))
            }
        }
    }
}

impl<D> Detokenizer for SorbitDetokenizer<D>
where
    D: Deserializer,
    Error: From<<D as Deserializer>::Error>,
{
    type Error = Error;

    fn ignore(&mut self, max_recursion: usize) -> Result<(), Self::Error> {
        let next = self.peek_token()?;
        match next {
            Token::TinyAtom(_) => self.read_token().map(|_| ()),
            Token::ShortAtom(_) => self.read_token().map(|_| ()),
            Token::MediumAtom(_) => self.read_token().map(|_| ()),
            Token::LongAtom(_) => self.read_token().map(|_| ()),
            Token::StartList => self.detokenize_list(|de| de.ignore(max_recursion - 1)).map(|_| ()),
            Token::EndList => Err(Error::UnexpectedEndList),
            Token::StartName => self
                .detokenize_named(|de| de.ignore(max_recursion - 1), |de, _| de.ignore(max_recursion - 1))
                .map(|_| ()),
            Token::EndName => Err(Error::UnexpectedEndNamed),
            Token::Call => self.read_token().map(|_| ()),
            Token::EndOfData => self.read_token().map(|_| ()),
            Token::EndOfSession => self.read_token().map(|_| ()),
            Token::StartTransaction => self.read_token().map(|_| ()),
            Token::EndTransaction => self.read_token().map(|_| ()),
            Token::Empty => self.read_token().map(|_| ()),
        }
    }

    fn peek_kind(&mut self) -> Result<TokenType, Self::Error> {
        let next = self.peek_token()?;
        Ok(TokenType::from(next))
    }

    fn detokenize_until<O>(
        &mut self,
        mut value: impl FnMut(&mut Self) -> Result<O, Self::Error>,
    ) -> Result<O, Self::Error> {
        loop {
            match value(self) {
                result @ Ok(_) => break result,
                result @ Err(Error::CanNotSerialize(_)) => break result,
                Err(_) => (),
            }
        }
    }

    fn detokenize_i8(&mut self) -> Result<i8, Error> {
        convert!(self.read_token()?, i8)
    }

    fn detokenize_i16(&mut self) -> Result<i16, Error> {
        convert!(self.read_token()?, i16)
    }

    fn detokenize_i32(&mut self) -> Result<i32, Error> {
        convert!(self.read_token()?, i32)
    }

    fn detokenize_i64(&mut self) -> Result<i64, Error> {
        convert!(self.read_token()?, i64)
    }

    fn detokenize_u8(&mut self) -> Result<u8, Error> {
        convert!(self.read_token()?, u8)
    }

    fn detokenize_u16(&mut self) -> Result<u16, Error> {
        convert!(self.read_token()?, u16)
    }

    fn detokenize_u32(&mut self) -> Result<u32, Error> {
        convert!(self.read_token()?, u32)
    }

    fn detokenize_u64(&mut self) -> Result<u64, Error> {
        convert!(self.read_token()?, u64)
    }

    fn detokenize_command(&mut self) -> Result<Command, Error> {
        convert!(self.read_token()?, Command)
    }

    fn detokenize_named<Name, Value>(
        &mut self,
        name: impl FnOnce(&mut Self) -> Result<Name, Error>,
        value: impl FnOnce(&mut Self, &Name) -> Result<Value, Error>,
    ) -> Result<(Name, Value), Error> {
        match self.read_token()? {
            Token::StartName => (),
            unexpected => return Err(Error::CanNotConvert { from: TokenType::from(&unexpected).into(), to: "named" }),
        };
        let name = name(self)?;
        let value = value(self, &name)?;
        if self.read_token()? != Token::EndName {
            return Err(Error::ExpectedEndNamed);
        };
        Ok((name, value))
    }

    fn detokenize_list(&mut self, mut item: impl FnMut(&mut Self) -> Result<(), Error>) -> Result<(), Error> {
        match self.read_token()? {
            Token::StartList => (),
            unexpected => return Err(Error::CanNotConvert { from: TokenType::from(&unexpected).into(), to: "list" }),
        };
        while self.peek_token()? != &Token::EndList {
            item(self)?;
        }
        let _ = self.read_token();
        Ok(())
    }

    fn detokenize_bytes(&mut self) -> Result<Vec<u8>, Error> {
        convert!(self.read_token()?, Vec<u8>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::Named;
    use crate::token::{Detokenize, FromTokens, ToTokens};

    use googletest::assert_that;
    use googletest::matchers::*;
    use rstest::rstest;
    use sorbit::{io::FixedMemoryStream, stream_ser_de::StreamDeserializer};

    #[rstest]
    #[case(3, &[0x03])]
    #[case(254, &[0b1000_0001, 254])]
    fn tokenize_u8(#[case] value: u8, #[case] bytes: &[u8]) {
        assert_eq!(value.to_tokens().unwrap(), bytes);
        assert_eq!(u8::from_tokens(bytes).unwrap(), value);
    }

    #[rstest]
    #[case(3, &[0x03])]
    #[case(0xABCD, &[0b1000_0010, 0xAB, 0xCD])]
    fn tokenize_u16(#[case] value: u16, #[case] bytes: &[u8]) {
        assert_eq!(value.to_tokens().unwrap(), bytes);
        assert_eq!(u16::from_tokens(bytes).unwrap(), value);
    }

    #[rstest]
    #[case(3, &[0x03])]
    #[case(0xABCDABCD, &[0b1000_0100, 0xAB, 0xCD, 0xAB, 0xCD])]
    fn tokenize_u32(#[case] value: u32, #[case] bytes: &[u8]) {
        assert_eq!(value.to_tokens().unwrap(), bytes);
        assert_eq!(u32::from_tokens(bytes).unwrap(), value);
    }

    #[rstest]
    #[case(3, &[0x03])]
    #[case(0xABCDABCDBBCDABCD, &[0b1000_1000, 0xAB, 0xCD, 0xAB, 0xCD, 0xBB, 0xCD, 0xAB, 0xCD])]
    fn tokenize_u64(#[case] value: u64, #[case] bytes: &[u8]) {
        assert_eq!(value.to_tokens().unwrap(), bytes);
        assert_eq!(u64::from_tokens(bytes).unwrap(), value);
    }

    #[rstest]
    #[case(-3, &[0b01_111101])]
    #[case(127, &[0b1001_0001, 127])]
    fn tokenize_i8(#[case] value: i8, #[case] bytes: &[u8]) {
        assert_eq!(value.to_tokens().unwrap(), bytes);
        assert_eq!(i8::from_tokens(bytes).unwrap(), value);
    }

    #[rstest]
    #[case(-3, &[0b01_111101])]
    #[case(0x0BCD, &[0b1001_0010, 0x0B, 0xCD])]
    fn tokenize_i16(#[case] value: i16, #[case] bytes: &[u8]) {
        assert_eq!(value.to_tokens().unwrap(), bytes);
        assert_eq!(i16::from_tokens(bytes).unwrap(), value);
    }

    #[rstest]
    #[case(-3, &[0b01_111101])]
    #[case(0x0BCDABCD, &[0b1001_0100, 0x0B, 0xCD, 0xAB, 0xCD])]
    fn tokenize_i32(#[case] value: i32, #[case] bytes: &[u8]) {
        assert_eq!(value.to_tokens().unwrap(), bytes);
        assert_eq!(i32::from_tokens(bytes).unwrap(), value);
    }

    #[rstest]
    #[case(-3, &[0b01_111101])]
    #[case(0x0BCDABCDBBCDABCD, &[0b1001_1000, 0x0B, 0xCD, 0xAB, 0xCD, 0xBB, 0xCD, 0xAB, 0xCD])]
    fn tokenize_i64(#[case] value: i64, #[case] bytes: &[u8]) {
        assert_eq!(value.to_tokens().unwrap(), bytes);
        assert_eq!(i64::from_tokens(bytes).unwrap(), value);
    }

    #[rstest]
    #[case(Command::Call, &[0xF8])]
    #[case(Command::EndOfData, &[0xF9])]
    #[case(Command::EndOfSession, &[0xFA])]
    #[case(Command::Empty, &[0xFF])]
    fn tokenize_command(#[case] value: Command, #[case] bytes: &[u8]) {
        assert_eq!(value.to_tokens().unwrap(), bytes);
        assert_eq!(Command::from_tokens(bytes).unwrap(), value);
    }

    #[rstest]
    #[case(Named{name: 1, value: 2}, &[0xF2, 1, 2, 0xF3])]
    fn tokenize_named(#[case] value: Named<u8, u8>, #[case] bytes: &[u8]) {
        assert_eq!(value.to_tokens().unwrap(), bytes);
        assert_eq!(Named::<_, _>::from_tokens(bytes).unwrap(), value);
    }

    #[rstest]
    #[case(vec![1, 2, 3], &[0xF0, 1, 2, 3, 0xF1])]
    fn tokenize_list(#[case] value: Vec<u8>, #[case] bytes: &[u8]) {
        assert_eq!(value.to_tokens().unwrap(), bytes);
        assert_eq!(Vec::<u8>::from_tokens(bytes).unwrap(), value);
    }

    #[test]
    fn detokenize_until_found() {
        let bytes = &[0xF0, 1, 2, 3, 0xF1, 0xF9];
        let mut detokenizer = SorbitDetokenizer::new(StreamDeserializer::new(FixedMemoryStream::new(bytes)));
        assert_eq!(detokenizer.detokenize_until(|de| Command::detokenize(de)), Ok(Command::EndOfData));
    }

    #[test]
    fn detokenize_until_missing() {
        let bytes = &[0xF0, 1, 2, 3, 0xF1, 1];
        let mut detokenizer = SorbitDetokenizer::new(StreamDeserializer::new(FixedMemoryStream::new(bytes)));
        assert_that!(
            detokenizer.detokenize_until(|de| Command::detokenize(de)),
            err(matches_pattern!(Error::CanNotSerialize(_)))
        );
    }
}
