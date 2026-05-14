//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use super::stream::{InputStream, OutputStream};

#[deprecated = "serialization is being replaced by sorbit, which is much more organized than this ad-hoc system"]
pub trait Serialize<Item> {
    type Error: core::error::Error;
    fn serialize(&self, stream: &mut OutputStream<Item>) -> Result<(), Self::Error>;
}

#[deprecated = "serialization is being replaced by sorbit, which is much more organized than this ad-hoc system"]
pub trait Deserialize<Item>
where
    Self: Sized,
{
    type Error: core::error::Error;
    fn deserialize(stream: &mut InputStream<Item>) -> Result<Self, Self::Error>;
}

pub trait SerializeBinary: Sized {
    type Error;
    fn to_bytes(&self) -> Result<Vec<u8>, Self::Error>;
}

pub trait DeserializeBinary: Sized {
    type Error;
    fn from_bytes(bytes: Vec<u8>) -> Result<Self, Self::Error>;
}

impl<T: Serialize<u8>> SerializeBinary for T {
    type Error = <T as Serialize<u8>>::Error;
    fn to_bytes(&self) -> Result<Vec<u8>, Self::Error> {
        let mut stream = OutputStream::<u8>::new();
        self.serialize(&mut stream).map(|_| stream.take())
    }
}

impl<T: Deserialize<u8>> DeserializeBinary for T {
    type Error = <T as Deserialize<u8>>::Error;
    fn from_bytes(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let mut stream = InputStream::from(bytes);
        Self::deserialize(&mut stream)
    }
}
