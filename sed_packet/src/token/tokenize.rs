use sorbit::io::{FixedMemoryStream, GrowingMemoryStream};
use sorbit::stream_ser_de::{StreamDeserializer, StreamSerializer};

use crate::token::MessageError;

use super::command::Command;
use super::error::Error;

pub trait Tokenize {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error>;
}

pub trait Detokenize: Sized {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error>;
}

pub trait ToTokens {
    fn to_tokens(&self) -> Result<Vec<u8>, Error>;
}

pub trait FromTokens: Sized {
    fn from_tokens(tokens: &[u8]) -> Result<Self, Error>;
}

impl<T> ToTokens for T
where
    T: Tokenize,
{
    fn to_tokens(&self) -> Result<Vec<u8>, Error> {
        use super::SorbitTokenizer;

        let stream = GrowingMemoryStream::new();
        let serializer = StreamSerializer::new(stream);
        let mut tokenizer = SorbitTokenizer::new(serializer);
        self.tokenize(&mut tokenizer)?;
        Ok(tokenizer.take().take().take())
    }
}

impl<T> FromTokens for T
where
    T: Detokenize,
{
    fn from_tokens(tokens: &[u8]) -> Result<Self, Error> {
        use super::SorbitDetokenizer;

        let stream = FixedMemoryStream::new(tokens);
        let serializer = StreamDeserializer::new(stream);
        let mut tokenizer = SorbitDetokenizer::new(serializer);
        T::detokenize(&mut tokenizer)
    }
}

pub trait Tokenizer {
    type Error: MessageError;

    fn tokenize_i8(&mut self, value: i8) -> Result<(), Self::Error>;
    fn tokenize_i16(&mut self, value: i16) -> Result<(), Self::Error>;
    fn tokenize_i32(&mut self, value: i32) -> Result<(), Self::Error>;
    fn tokenize_i64(&mut self, value: i64) -> Result<(), Self::Error>;
    fn tokenize_u8(&mut self, value: u8) -> Result<(), Self::Error>;
    fn tokenize_u16(&mut self, value: u16) -> Result<(), Self::Error>;
    fn tokenize_u32(&mut self, value: u32) -> Result<(), Self::Error>;
    fn tokenize_u64(&mut self, value: u64) -> Result<(), Self::Error>;
    fn tokenize_command(&mut self, value: Command) -> Result<(), Self::Error>;
    fn tokenize_named(&mut self, name: impl Tokenize, value: impl Tokenize) -> Result<(), Self::Error>;
    fn tokenize_list(&mut self, items: impl FnOnce(&mut Self) -> Result<(), Self::Error>) -> Result<(), Self::Error>;
    fn tokenize_bytes(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;
}

pub trait Detokenizer {
    type Error: MessageError;

    fn ignore(&mut self, max_recursion: usize) -> Result<(), Self::Error>;
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
    fn detokenize_list(&mut self, item: impl FnMut(&mut Self) -> Result<(), Self::Error>) -> Result<(), Self::Error>;
    fn detokenize_bytes(&mut self) -> Result<Vec<u8>, Self::Error>;
}

impl<V: Tokenize> Tokenize for &V {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        (*self).tokenize(tokenizer)
    }
}
