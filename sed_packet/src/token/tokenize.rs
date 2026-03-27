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
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error>;
}

pub trait Detokenize: Sized {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error>;
}

pub trait Tokenizer {
    type Error;

    fn tokenize_i8(&mut self, value: i8) -> Result<(), Self::Error>;
    fn tokenize_i16(&mut self, value: i32) -> Result<(), Self::Error>;
    fn tokenize_i32(&mut self, value: i32) -> Result<(), Self::Error>;
    fn tokenize_i64(&mut self, value: i64) -> Result<(), Self::Error>;
    fn tokenize_u8(&mut self, value: u8) -> Result<(), Self::Error>;
    fn tokenize_u16(&mut self, value: u32) -> Result<(), Self::Error>;
    fn tokenize_u32(&mut self, value: u32) -> Result<(), Self::Error>;
    fn tokenize_u64(&mut self, value: u64) -> Result<(), Self::Error>;
    fn tokenize_command(&mut self, value: Command) -> Result<(), Self::Error>;
    fn tokenize_named(&mut self, name: impl Tokenize, value: impl Tokenize) -> Result<(), Self::Error>;
    fn tokenize_list(&mut self, items: impl FnOnce(&mut Self) -> Result<(), Self::Error>) -> Result<(), Self::Error>;
    fn tokenize_bytes(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;
}

pub trait Detokenizer {
    type Error;

    fn detokenize_i8(&mut self) -> Result<i8, Self::Error>;
    fn detokenize_i16(&mut self) -> Result<i16, Self::Error>;
    fn detokenize_i32(&mut self) -> Result<i32, Self::Error>;
    fn detokenize_i64(&mut self) -> Result<i64, Self::Error>;
    fn detokenize_u8(&mut self) -> Result<u8, Self::Error>;
    fn detokenize_u16(&mut self) -> Result<u16, Self::Error>;
    fn detokenize_u32(&mut self) -> Result<u32, Self::Error>;
    fn detokenize_u64(&mut self) -> Result<u64, Self::Error>;
    fn detokenize_command(&mut self) -> Result<Command, Self::Error>;
    fn detokenize_named<Name, Value>(
        &mut self,
        name: impl FnOnce(&mut Self) -> Result<Name, Self::Error>,
        value: impl FnOnce(&mut Self, &Name) -> Result<Value, Self::Error>,
    ) -> Result<(Name, Value), Self::Error>;
    fn detokenize_list(&mut self, item: impl Fn(&mut Self) -> Result<(), Self::Error>) -> Result<(), Self::Error>;
    fn detokenize_bytes(&mut self) -> Result<Vec<u8>, Self::Error>;
}

pub struct SorbitTokenizer<S>
where
    S: Serializer<Error = SorbitError>,
{
    serializer: S,
}

impl<S> SorbitTokenizer<S>
where
    S: Serializer<Error = SorbitError>,
{
    pub fn new(serializer: S) -> Self {
        Self { serializer }
    }
}

impl<S> Tokenizer for SorbitTokenizer<S>
where
    S: Serializer<Error = SorbitError>,
{
    type Error = Error;

    fn tokenize_i8(&mut self, value: i8) -> Result<(), Error> {
        Token::from(value).serialize(&mut self.serializer)?;
        Ok(())
    }

    fn tokenize_i16(&mut self, value: i32) -> Result<(), Error> {
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

    fn tokenize_u16(&mut self, value: u32) -> Result<(), Error> {
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
        Token::StartName.serialize(&mut self.serializer)?;
        Ok(())
    }

    fn tokenize_list(&mut self, items: impl FnOnce(&mut Self) -> Result<(), Error>) -> Result<(), Error> {
        Token::StartList.serialize(&mut self.serializer)?;
        items(self)?;
        Token::EndList.serialize(&mut self.serializer)?;
        Ok(())
    }

    fn tokenize_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
        Token::try_from(bytes).map_err(|_| Error::OversizedPayload)?.serialize(&mut self.serializer)?;
        Ok(())
    }
}

pub struct SorbitDetokenizer<D>
where
    D: Deserializer<Error = SorbitError>,
{
    deserializer: D,
}

impl<D> SorbitDetokenizer<D>
where
    D: Deserializer<Error = SorbitError>,
{
    pub fn new(deserializer: D) -> Self {
        Self { deserializer }
    }

    fn next_token(&mut self) -> Result<Token, Error> {
        Token::deserialize(&mut self.deserializer).map_err(|e| Error::SerializationFailed(e))
    }
}

impl<D> Detokenizer for SorbitDetokenizer<D>
where
    D: Deserializer<Error = SorbitError>,
{
    type Error = Error;

    fn detokenize_i8(&mut self) -> Result<i8, Error> {
        self.next_token()?.try_into().map_err(|_| Error::InvalidDataType)
    }

    fn detokenize_i16(&mut self) -> Result<i16, Error> {
        self.next_token()?.try_into().map_err(|_| Error::InvalidDataType)
    }

    fn detokenize_i32(&mut self) -> Result<i32, Error> {
        self.next_token()?.try_into().map_err(|_| Error::InvalidDataType)
    }

    fn detokenize_i64(&mut self) -> Result<i64, Error> {
        self.next_token()?.try_into().map_err(|_| Error::InvalidDataType)
    }

    fn detokenize_u8(&mut self) -> Result<u8, Error> {
        self.next_token()?.try_into().map_err(|_| Error::InvalidDataType)
    }

    fn detokenize_u16(&mut self) -> Result<u16, Error> {
        self.next_token()?.try_into().map_err(|_| Error::InvalidDataType)
    }

    fn detokenize_u32(&mut self) -> Result<u32, Error> {
        self.next_token()?.try_into().map_err(|_| Error::InvalidDataType)
    }

    fn detokenize_u64(&mut self) -> Result<u64, Error> {
        self.next_token()?.try_into().map_err(|_| Error::InvalidDataType)
    }

    fn detokenize_command(&mut self) -> Result<Command, Error> {
        self.next_token()?.try_into().map_err(|_| Error::InvalidDataType)
    }

    fn detokenize_named<Name, Value>(
        &mut self,
        name: impl FnOnce(&mut Self) -> Result<Name, Error>,
        value: impl FnOnce(&mut Self, &Name) -> Result<Value, Error>,
    ) -> Result<(Name, Value), Error> {
        if self.next_token()? != Token::StartName {
            return Err(Error::InvalidNamedDelimiter);
        };
        let name = name(self)?;
        let value = value(self, &name)?;
        if self.next_token()? != Token::EndName {
            return Err(Error::InvalidNamedDelimiter);
        };
        Ok((name, value))
    }

    fn detokenize_list(&mut self, item: impl Fn(&mut Self) -> Result<(), Error>) -> Result<(), Error> {
        if self.next_token()? != Token::StartName {
            return Err(Error::InvalidListDelimiter);
        };
        while self.next_token()? != Token::EndName {
            item(self)?;
        }
        Ok(())
    }

    fn detokenize_bytes(&mut self) -> Result<Vec<u8>, Error> {
        self.next_token()?.try_into().map_err(|_| Error::InvalidDataType)
    }
}
