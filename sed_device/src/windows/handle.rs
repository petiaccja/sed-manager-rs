use std::path::Path;

use windows::Win32::Foundation::*;
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::Storage::FileSystem::*;
use windows::core::HSTRING;

use crate::Error;

pub struct Handle(HANDLE);

impl Handle {
    pub fn open(path: impl AsRef<Path>, overlapped: bool) -> Result<Self, Error> {
        let path_utf16 = HSTRING::from(path.as_ref().as_os_str());
        let flags = if overlapped {
            FILE_FLAG_OVERLAPPED
        } else {
            FILE_FLAGS_AND_ATTRIBUTES(0)
        };
        let handle = unsafe {
            CreateFileW(
                &path_utf16,
                (GENERIC_READ | GENERIC_WRITE | GENERIC_EXECUTE).0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                flags,
                None,
            )?
        };
        Ok(Self(handle))
    }

    pub fn inner(&self) -> HANDLE {
        self.0
    }
}

impl From<HANDLE> for Handle {
    fn from(value: HANDLE) -> Self {
        Self(value)
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        if self.0 != INVALID_HANDLE_VALUE {
            unsafe {
                let _ = CloseHandle(self.0);
            };
        }
    }
}

unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}
