//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::os::fd::OwnedFd;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rustix::fs::{Mode, OFlags};
use rustix::ioctl::Ioctl;

use crate::Error;

pub struct IoctlDevice {
    fd: Arc<OwnedFd>,
    path: PathBuf,
}

impl IoctlDevice {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref().to_owned();
        let open_path = path.clone();
        let fd = blocking::unblock(move || rustix::fs::open(&open_path, OFlags::RDWR, Mode::empty())).await?;
        Ok(Self { fd: Arc::new(fd), path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub async fn ioctl<I, Output>(&self, ioctl: I) -> Result<Output, Error>
    where
        I: Ioctl<Output = Output> + Send + 'static,
        Output: Send + 'static,
    {
        let fd = self.fd.clone();
        blocking::unblock(move || unsafe { rustix::ioctl::ioctl(fd, ioctl) }).await.map_err(Error::from)
    }
}
