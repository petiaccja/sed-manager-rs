//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::ptr::null_mut;

use winapi::{
    shared::ntdef::HANDLE,
    um::{
        fileapi::{CreateFileW, OPEN_EXISTING},
        handleapi::{CloseHandle, INVALID_HANDLE_VALUE},
        winnt::{FILE_SHARE_READ, FILE_SHARE_WRITE, GENERIC_EXECUTE, GENERIC_READ, GENERIC_WRITE},
    },
};

use crate::device::{shared::string::ToNullTerminated, Error};

use crate::device::windows::error::get_last_error;

pub struct FileHandle {
    handle: HANDLE,
    path: String,
}

unsafe impl Send for FileHandle {}

unsafe impl Sync for FileHandle {}

impl FileHandle {
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

    pub fn handle(&self) -> HANDLE {
        self.handle
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

impl Drop for FileHandle {
    fn drop(&mut self) {
        if self.handle != INVALID_HANDLE_VALUE {
            unsafe {
                CloseHandle(self.handle);
            };
        }
    }
}
