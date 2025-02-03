mod binary_impl;
mod error;
mod fields;
mod serialize;
mod stream;
pub mod vec_with_len;
pub mod vec_without_len;

pub use error::{annotate_field, Error};
pub use fields::{deserialize_field, extend_with_zeros_until, serialize_field};
pub use sed_manager_macros::{Deserialize, Serialize};
pub use serialize::{Deserialize, DeserializeBinary, Serialize, SerializeBinary};
pub use stream::{InputStream, ItemRead, ItemWrite, OutputStream, SeekAlways};
