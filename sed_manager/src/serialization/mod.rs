//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

mod binary_impl;
mod error;
pub mod field;
mod serialize;
mod stream;
pub mod vec_with_len;
pub mod vec_without_len;

pub use error::{Error, annotate_field};
#[deprecated = "serialization is being replaced by sorbit, which is much more organized than this ad-hoc system"]
pub use sed_manager_macros::{Deserialize, Serialize};
pub use serialize::{Deserialize, DeserializeBinary, Serialize, SerializeBinary};
pub use stream::{ByteOrder, InputStream, ItemRead, ItemWrite, OutputStream, Seek, SeekFrom};

// Serialization and deserialization using sorbit.
// These will reside here while the system is replaced to use sorbit.

use sorbit::io::{FixedMemoryStream, GrowingMemoryStream};
use sorbit::stream_ser_de::{StreamDeserializer, StreamSerializer};

pub trait SerializeBinarySorbit: Sized {
    type Error;
    fn to_bytes(&self) -> Result<Vec<u8>, Self::Error>;
}

pub trait DeserializeBinarySorbit<'buffer>: Sized {
    type Error;
    fn from_bytes(bytes: &'buffer [u8]) -> Result<Self, Self::Error>;
}

impl<T: sorbit::ser_de::Serialize> SerializeBinarySorbit for T {
    type Error = <StreamSerializer<GrowingMemoryStream> as sorbit::ser_de::SerializationOutcome>::Error;
    fn to_bytes(&self) -> Result<Vec<u8>, Self::Error> {
        let mut serializer = StreamSerializer::new(GrowingMemoryStream::new());
        self.serialize(&mut serializer)?;
        Ok(serializer.take().take())
    }
}

impl<'buffer, T: sorbit::ser_de::Deserialize> DeserializeBinarySorbit<'buffer> for T {
    type Error = <StreamDeserializer<FixedMemoryStream<&'buffer [u8]>> as sorbit::ser_de::Deserializer>::Error;
    fn from_bytes(bytes: &'buffer [u8]) -> Result<Self, Self::Error> {
        let mut deserializer = StreamDeserializer::new(FixedMemoryStream::new(bytes));
        Self::deserialize(&mut deserializer)
    }
}
