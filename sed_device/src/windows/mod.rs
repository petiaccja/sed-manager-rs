//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

mod devices;
mod drive_list;

pub use devices::open_device;
pub use drive_list::list_physical_drives;
