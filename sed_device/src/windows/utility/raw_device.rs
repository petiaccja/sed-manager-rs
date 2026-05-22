//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::ptr::null_mut;
use std::ffi::c_void;

use winapi::{
    shared::{
        minwindef::{DWORD, FALSE},
        ntdef::HANDLE,
    },
    um::{
        fileapi::{CreateFileW, OPEN_EXISTING},
        handleapi::{CloseHandle, INVALID_HANDLE_VALUE},
        ioapiset::DeviceIoControl,
        winnt::{FILE_SHARE_READ, FILE_SHARE_WRITE, GENERIC_EXECUTE, GENERIC_READ, GENERIC_WRITE},
    },
};

use crate::{Error, shared::string::ToNullTerminated};

use crate::windows::error::get_last_error;

pub struct RawDevice {
    handle: HANDLE,
    path: String,
}

unsafe impl Send for RawDevice {}
unsafe impl Sync for RawDevice {}

impl RawDevice {
    pub fn open(path: &str) -> Result<Self, Error> {
        let mut file_name_utf16: Vec<u16> = path.to_null_terminated_utf16();
        unsafe {
            let handle = CreateFileW(
                file_name_utf16.as_mut_ptr(),
                GENERIC_READ | GENERIC_WRITE | GENERIC_EXECUTE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                null_mut(),
                OPEN_EXISTING,
                0,
                null_mut(),
            );
            if handle == INVALID_HANDLE_VALUE {
                get_last_error()?;
            };
            Ok(Self { handle, path: path.into() })
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub async fn device_io_control_symmetric(&self, ioctl: DWORD, buffer: &mut [u8]) -> Result<u32, Error> {
        let mut bytes_returned: u32 = 0;
        let result = unsafe {
            DeviceIoControl(
                self.handle,
                ioctl,
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len() as u32,
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len() as u32,
                &mut bytes_returned as *mut u32,
                null_mut(),
            )
        };

        if result == FALSE {
            get_last_error()?;
        };
        Ok(bytes_returned)
    }

    pub async fn device_io_control(
        &self,
        ioctl: DWORD,
        mut request_buffer: Option<&mut [u8]>,
        mut response_buffer: Option<&mut [u8]>,
    ) -> Result<u32, Error> {
        let mut bytes_returned: u32 = 0;
        let result = unsafe {
            DeviceIoControl(
                self.handle,
                ioctl,
                request_buffer.as_mut().map(|buf| buf.as_mut_ptr() as *mut c_void).unwrap_or(null_mut()),
                request_buffer.as_ref().map(|buf| buf.len() as u32).unwrap_or(0),
                response_buffer.as_mut().map(|buf| buf.as_mut_ptr() as *mut c_void).unwrap_or(null_mut()),
                response_buffer.as_ref().map(|buf| buf.len() as u32).unwrap_or(0),
                &mut bytes_returned as *mut u32,
                null_mut(),
            )
        };

        if result == FALSE {
            get_last_error()?;
        };
        Ok(bytes_returned)
    }
}

impl Drop for RawDevice {
    fn drop(&mut self) {
        if self.handle != INVALID_HANDLE_VALUE {
            unsafe {
                CloseHandle(self.handle);
            };
        }
    }
}
