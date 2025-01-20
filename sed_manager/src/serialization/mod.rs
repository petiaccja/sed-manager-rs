mod binary_serialization;
mod error;
mod fields;
pub mod item_with_len;
mod serialize;
mod stream;
pub mod vec_with_len;
pub mod vec_without_len;

pub use binary_serialization::{DeserializeBinary, SerializeBinary};
pub use error::{annotate_field, Error};
pub use fields::{deserialize_field, extend_with_zeros_until, serialize_field};
pub use sed_manager_macros::{Deserialize, Serialize};
pub use serialize::{Deserialize, Serialize};
pub use stream::{InputStream, ItemRead, ItemWrite, OutputStream, SeekAlways};
