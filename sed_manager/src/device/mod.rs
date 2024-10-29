mod device;
mod nvme;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "windows")]
use windows as os;

#[cfg(target_os = "linux")]
use linux as os;

pub use device::{Device, DeviceError, Interface};
pub use os::{get_physical_drive_interface, get_physical_drives, NVMeDevice};
