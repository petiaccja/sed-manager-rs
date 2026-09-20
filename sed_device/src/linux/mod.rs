//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

mod device_list;
mod devices;
mod error;
mod ioctl_device;

pub use device_list::list_storage_devices;
pub use devices::open_storage_device;
pub use error::Error;
