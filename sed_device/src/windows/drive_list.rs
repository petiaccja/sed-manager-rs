//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::path::PathBuf;

use windows::{
    Win32::{
        Devices::DeviceAndDriverInstallation::{
            DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, SP_DEVICE_INTERFACE_DATA, SP_DEVICE_INTERFACE_DETAIL_DATA_W,
            SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW, SetupDiGetDeviceInterfaceDetailW,
        },
        Foundation::ERROR_NO_MORE_ITEMS,
        System::Ioctl::GUID_DEVINTERFACE_DISK,
    },
    core::{HRESULT, HSTRING, HStringBuilder, PCWSTR},
};

use crate::Error as DeviceError;

pub fn list_physical_drives() -> Result<Vec<PathBuf>, DeviceError> {
    let dev_info = unsafe {
        SetupDiGetClassDevsW(
            Some(&GUID_DEVINTERFACE_DISK as *const _),
            None,
            None,
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        )?
    };

    let mut device_paths = Vec::<PathBuf>::new();

    for disk_idx in 0.. {
        let mut iface_data =
            SP_DEVICE_INTERFACE_DATA { cbSize: size_of::<SP_DEVICE_INTERFACE_DATA>() as u32, ..Default::default() };
        match unsafe {
            SetupDiEnumDeviceInterfaces(
                dev_info,
                None,
                &GUID_DEVINTERFACE_DISK as *const _,
                disk_idx,
                &mut iface_data as *mut _,
            )
        } {
            Ok(_) => (),
            Err(err) if err.code() == HRESULT::from_win32(ERROR_NO_MORE_ITEMS.0) => break,
            Err(err) => return Err(err.into()),
        }

        let mut required_size = 0u32;
        unsafe {
            let _ = SetupDiGetDeviceInterfaceDetailW(
                dev_info,
                &mut iface_data as *mut _,
                None,
                0,
                Some(&mut required_size as *mut _),
                None,
            );
        };

        // This is a `SP_DEVICE_INTERFACE_DETAIL_DATA_W` allocated as a vector,
        // with extra space at the end.
        let mut detail_buffer = vec![0u8; required_size as usize];

        let detail = unsafe { &mut *(detail_buffer.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W) };
        detail.cbSize = size_of_val(detail) as u32;

        unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                dev_info,
                &mut iface_data as *mut _,
                Some(detail as *mut _),
                detail_buffer.len() as u32,
                None,
                None,
            )?
        };

        let device_path = PCWSTR(detail.DevicePath.as_ptr());
        device_paths.push(HSTRING::from(HStringBuilder::new(unsafe { device_path.len() })).to_os_string().into());
    }

    Ok(device_paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_physical_drives() -> Result<(), DeviceError> {
        // There must be at least one physical drive, so this test should pass.
        match list_physical_drives() {
            Ok(physical_drives) => {
                assert!(!physical_drives.is_empty());
                Ok(())
            }
            Err(DeviceError::PermissionDenied) => Ok(()),
            Err(err) => Err(err),
        }
    }
}
