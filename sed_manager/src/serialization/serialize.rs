use super::stream::{InputStream, OutputStream};

pub trait Serialize<T, Item> {
    type Error: std::error::Error;
    fn serialize(&self, stream: &mut OutputStream<Item>) -> Result<(), Self::Error>;
}

pub trait Deserialize<T, Item> {
    type Error: std::error::Error;
    fn deserialize(stream: &mut InputStream<Item>) -> Result<T, Self::Error>;
}
