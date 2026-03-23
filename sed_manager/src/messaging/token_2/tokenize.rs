use sorbit::error::Error as SorbitError;
use sorbit::ser_de::{Deserialize as _, Deserializer, Serialize as _, Serializer};

use super::command::Command;
use super::token::Token;

pub enum Error {
    SerializationFailed(SorbitError),
    OversizedPayload,
    InvalidDataType,
    InvalidNamedDelimiter,
    InvalidListDelimiter,
}

impl From<SorbitError> for Error {
    fn from(value: SorbitError) -> Self {
        Self::SerializationFailed(value)
    }
}

pub trait Tokenize {
    fn tokenize<S>(&self, tokenizer: &mut Tokenizer<S>) -> Result<(), Error>
    where
        S: Serializer<Error = SorbitError>;
}

pub trait Detokenize {
    fn detokenize<D>(detokenizer: &mut Detokenizer<D>) -> Result<Self, Error>
    where
        Self: Sized,
        D: Deserializer<Error = SorbitError>;
}

pub struct Tokenizer<S>
where
    S: Serializer<Error = SorbitError>,
{
    serializer: S,
}

impl<S> Tokenizer<S>
where
    S: Serializer<Error = SorbitError>,
{
    pub fn new(serializer: S) -> Self {
        Self { serializer }
    }

    pub fn tokenize_i8(&mut self, value: i8) -> Result<(), Error> {
        Token::from(value).serialize(&mut self.serializer)?;
        Ok(())
    }

    pub fn tokenize_i16(&mut self, value: i32) -> Result<(), Error> {
        Token::from(value).serialize(&mut self.serializer)?;
        Ok(())
    }

    pub fn tokenize_i32(&mut self, value: i32) -> Result<(), Error> {
        Token::from(value).serialize(&mut self.serializer)?;
        Ok(())
    }

    pub fn tokenize_i64(&mut self, value: i64) -> Result<(), Error> {
        Token::from(value).serialize(&mut self.serializer)?;
        Ok(())
    }

    pub fn tokenize_u8(&mut self, value: u8) -> Result<(), Error> {
        Token::from(value).serialize(&mut self.serializer)?;
        Ok(())
    }

    pub fn tokenize_u16(&mut self, value: u32) -> Result<(), Error> {
        Token::from(value).serialize(&mut self.serializer)?;
        Ok(())
    }

    pub fn tokenize_u32(&mut self, value: u32) -> Result<(), Error> {
        Token::from(value).serialize(&mut self.serializer)?;
        Ok(())
    }

    pub fn tokenize_u64(&mut self, value: u64) -> Result<(), Error> {
        Token::from(value).serialize(&mut self.serializer)?;
        Ok(())
    }

    pub fn tokenize_command(&mut self, value: Command) -> Result<(), Error> {
        Token::from(value).serialize(&mut self.serializer)?;
        Ok(())
    }

    pub fn tokenize_named(&mut self, name: impl Tokenize, value: impl Tokenize) -> Result<(), Error> {
        Token::StartName.serialize(&mut self.serializer)?;
        name.tokenize(self)?;
        value.tokenize(self)?;
        Token::StartName.serialize(&mut self.serializer)?;
        Ok(())
    }

    pub fn tokenize_list(&mut self, items: impl FnOnce(&mut Self) -> Result<(), Error>) -> Result<(), Error> {
        Token::StartList.serialize(&mut self.serializer)?;
        items(self)?;
        Token::EndList.serialize(&mut self.serializer)?;
        Ok(())
    }

    pub fn tokenize_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
        Token::try_from(bytes).map_err(|_| Error::OversizedPayload)?.serialize(&mut self.serializer)?;
        Ok(())
    }
}

pub struct Detokenizer<D>
where
    D: Deserializer<Error = SorbitError>,
{
    deserializer: D,
}

impl<D> Detokenizer<D>
where
    D: Deserializer<Error = SorbitError>,
{
    pub fn new(deserializer: D) -> Self {
        Self { deserializer }
    }

    fn next_token(&mut self) -> Result<Token, Error> {
        Token::deserialize(&mut self.deserializer).map_err(|e| Error::SerializationFailed(e))
    }

    pub fn detokenize_i8(&mut self) -> Result<i8, Error> {
        self.next_token()?.try_into().map_err(|_| Error::InvalidDataType)
    }

    pub fn detokenize_i16(&mut self) -> Result<i16, Error> {
        self.next_token()?.try_into().map_err(|_| Error::InvalidDataType)
    }

    pub fn detokenize_i32(&mut self) -> Result<i32, Error> {
        self.next_token()?.try_into().map_err(|_| Error::InvalidDataType)
    }

    pub fn detokenize_i64(&mut self) -> Result<i64, Error> {
        self.next_token()?.try_into().map_err(|_| Error::InvalidDataType)
    }

    pub fn detokenize_u8(&mut self) -> Result<u8, Error> {
        self.next_token()?.try_into().map_err(|_| Error::InvalidDataType)
    }

    pub fn detokenize_u16(&mut self) -> Result<u16, Error> {
        self.next_token()?.try_into().map_err(|_| Error::InvalidDataType)
    }

    pub fn detokenize_u32(&mut self) -> Result<u32, Error> {
        self.next_token()?.try_into().map_err(|_| Error::InvalidDataType)
    }

    pub fn detokenize_u64(&mut self) -> Result<u64, Error> {
        self.next_token()?.try_into().map_err(|_| Error::InvalidDataType)
    }

    pub fn detokenize_command(&mut self) -> Result<Command, Error> {
        self.next_token()?.try_into().map_err(|_| Error::InvalidDataType)
    }

    pub fn detokenize_named<Name, Value>(
        &mut self,
        name: impl FnOnce(&mut Self) -> Result<Name, Error>,
        value: impl FnOnce(&mut Self) -> Result<Value, Error>,
    ) -> Result<(Name, Value), Error> {
        if self.next_token()? != Token::StartName {
            return Err(Error::InvalidNamedDelimiter);
        };
        let name = name(self)?;
        let value = value(self)?;
        if self.next_token()? != Token::EndName {
            return Err(Error::InvalidNamedDelimiter);
        };
        Ok((name, value))
    }

    pub fn detokenize_list(&mut self, item: impl Fn(&mut Self) -> Result<(), Error>) -> Result<(), Error> {
        if self.next_token()? != Token::StartName {
            return Err(Error::InvalidListDelimiter);
        };
        while self.next_token()? != Token::EndName {
            item(self)?;
        }
        Ok(())
    }

    pub fn detokenize_bytes(&mut self) -> Result<Vec<u8>, Error> {
        self.next_token()?.try_into().map_err(|_| Error::InvalidDataType)
    }
}
