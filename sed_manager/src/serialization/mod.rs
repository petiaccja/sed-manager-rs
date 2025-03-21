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

pub use error::{annotate_field, Error};
pub use sed_manager_macros::{Deserialize, Serialize};
pub use serialize::{Deserialize, DeserializeBinary, Serialize, SerializeBinary};
pub use stream::{ByteOrder, InputStream, ItemRead, ItemWrite, OutputStream, Seek, SeekFrom};
