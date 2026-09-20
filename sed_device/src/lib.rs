//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

mod error;
#[cfg(feature = "test-utils")]
pub mod mock_device;
mod shared;
mod storage_device;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "windows")]
use windows as os;

#[cfg(target_os = "linux")]
use linux as os;

pub use error::Error;
pub use os::{list_storage_devices, open_storage_device};
pub use storage_device::{Interface, StorageDevice};
