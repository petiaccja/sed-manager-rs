//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use windows::Win32::System::IO::*;

use crate::Error;
use crate::windows::handle::Handle;
use crate::windows::thread_pool::{ThreadPoolIo, spawn};

pub struct IoctlDevice {
    thread_pool_io: ThreadPoolIo,
    handle: Handle,
    path: PathBuf,
}

impl IoctlDevice {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref().to_owned();
        let result = spawn(move || {
            let handle = Handle::open(&path, true)?;
            let tpio = ThreadPoolIo::new(handle.inner())?;
            Ok(Self { handle, thread_pool_io: tpio, path })
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

    pub async fn ioctl_symmetric(&self, control_code: u32, buffer: &mut [u8]) -> Result<u32, Error> {
        self.thread_pool_io
            .spawn(|overlapped| {
                let mut bytes_returned: u32 = 0;
                unsafe {
                    DeviceIoControl(
                        self.handle.inner(),
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

    pub async fn ioctl(
        &self,
        control_code: u32,
        request_buffer: Option<&[u8]>,
        mut response_buffer: Option<&mut [u8]>,
    ) -> Result<u32, Error> {
        self.thread_pool_io
            .spawn(|overlapped| {
                let mut bytes_returned: u32 = 0;

                unsafe {
                    DeviceIoControl(
                        self.handle.inner(),
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
