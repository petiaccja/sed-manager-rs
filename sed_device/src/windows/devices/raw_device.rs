//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use windows::Win32::Foundation::*;
use windows::Win32::Storage::FileSystem::*;
use windows::Win32::System::IO::*;
use windows::core::HSTRING;

use crate::Error;
use crate::windows::async_io::{ThreadPoolIo, submit_work};
use crate::windows::device_handle::DeviceHandle;

pub struct RawDevice {
    tpio: ThreadPoolIo,
    handle: DeviceHandle,
    path: PathBuf,
}

unsafe impl Send for RawDevice {}
unsafe impl Sync for RawDevice {}

impl RawDevice {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref().to_owned();
        let path_utf16 = HSTRING::from(path.as_os_str());
        let result = submit_work(move || {
            let handle = DeviceHandle(unsafe {
                CreateFileW(
                    &path_utf16,
                    (GENERIC_READ | GENERIC_WRITE | GENERIC_EXECUTE).0,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    None,
                    OPEN_EXISTING,
                    FILE_FLAG_OVERLAPPED,
                    None,
                )?
            });
            let tpio = ThreadPoolIo::new(handle.0)?;
            Ok(Self { handle, tpio, path })
        })
        .await;
        match result {
            Ok(result) => result,
            Err(err) => Err(err.err_or_resume_unwind().into()),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub async fn device_io_control_symmetric(&self, control_code: u32, buffer: &mut [u8]) -> Result<u32, Error> {
        self.tpio
            .submit(|overlapped| {
                let mut bytes_returned: u32 = 0;
                unsafe {
                    DeviceIoControl(
                        self.handle.0,
                        control_code,
                        Some(buffer.as_ptr() as *const c_void),
                        buffer.len() as u32,
                        Some(buffer.as_mut_ptr() as *mut c_void),
                        buffer.len() as u32,
                        Some(&mut bytes_returned as *mut _),
                        Some(overlapped.load(Ordering::Relaxed)),
                    )
                }
                .map(|_| bytes_returned)
            })
            .await
            .map_err(|err| err.into())
    }

    pub async fn device_io_control(
        &self,
        control_code: u32,
        request_buffer: Option<&[u8]>,
        mut response_buffer: Option<&mut [u8]>,
    ) -> Result<u32, Error> {
        self.tpio
            .submit(|overlapped| {
                let mut bytes_returned: u32 = 0;

                unsafe {
                    DeviceIoControl(
                        self.handle.0,
                        control_code,
                        request_buffer.as_ref().map(|buf| buf.as_ptr() as *const c_void),
                        request_buffer.as_ref().map(|buf| buf.len() as u32).unwrap_or(0),
                        response_buffer.as_mut().map(|buf| buf.as_mut_ptr() as *mut c_void),
                        response_buffer.as_ref().map(|buf| buf.len() as u32).unwrap_or(0),
                        Some(&mut bytes_returned as *mut _),
                        Some(overlapped.load(Ordering::Relaxed)),
                    )
                }
                .map(|_| bytes_returned)
            })
            .await
            .map_err(|err| err.into())
    }
}
