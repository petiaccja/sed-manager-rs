use nix::{fcntl, libc::close};
use std::os::fd::RawFd;

use crate::device::linux::Error as LinuxError;

pub struct FileHandle {
    handle: RawFd,
    path: String,
}

unsafe impl Send for FileHandle {}
unsafe impl Sync for FileHandle {}

impl FileHandle {
    pub fn open(path: &str) -> Result<Self, LinuxError> {
        let handle = fcntl::open(path, fcntl::OFlag::O_RDWR, nix::sys::stat::Mode::all())?;
        Ok(Self { handle, path: path.to_string() })
    }

    pub fn handle(&self) -> RawFd {
        self.handle
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

impl Drop for FileHandle {
    fn drop(&mut self) {
        let _ = unsafe { close(self.handle) };
    }
}
