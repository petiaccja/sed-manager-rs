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

pub use device::{Device, Error, Interface};
pub use os::{get_drive_interface, list_physical_drives, NVMeDevice};
