use super::stream::{InputStream, OutputStream};

pub trait Serialize<Item> {
    type Error: std::error::Error;
    fn serialize(&self, stream: &mut OutputStream<Item>) -> Result<(), Self::Error>;
}

pub trait Deserialize<Item>
where
    Self: Sized,
{
    type Error: std::error::Error;
    fn deserialize(stream: &mut InputStream<Item>) -> Result<Self, Self::Error>;
}
