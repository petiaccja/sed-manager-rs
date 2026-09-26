//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

mod ioctl {
    use crate::{Error, linux::ioctl_device::IoctlDevice};

    /// `BLKSSZGET`. Returns the logical sector size of a block device, in bytes.
    /// Despite returning data, this is a legacy `_IO(0x12, 104)` opcode with no
    /// encoded size, so it must be composed with `opcode::none`, not `opcode::read`.
    const BLKSSZGET: rustix::ioctl::Opcode = rustix::ioctl::opcode::none(0x12, 104);
    /// `BLKGETSIZE64`. Returns the total size of a block device, in bytes.
    const BLKGETSIZE64: rustix::ioctl::Opcode = rustix::ioctl::opcode::read::<u64>(0x12, 114);

    pub trait GenericIoctlDevice {
        async fn logical_sector_size(&self) -> Result<u32, Error>;
        async fn logical_sector_count(&self) -> Result<u64, Error>;
    }

    impl GenericIoctlDevice for IoctlDevice {
        async fn logical_sector_size(&self) -> Result<u32, Error> {
            self.ioctl(unsafe { rustix::ioctl::Getter::<BLKSSZGET, u32>::new() }).await
        }

        /// Return the number of logical sectors on the device.
        ///
        /// # Errors
        ///
        /// In addition to common ioctl failures, this function may also return
        /// [`InvalidArgument`] if the retrieved sector size is zero, in which case
        /// the sector count can not be obtained.
        ///
        /// [`InvalidArgument`]: Error::InvalidArgument
        async fn logical_sector_count(&self) -> Result<u64, Error> {
            let sector_size = u64::from(self.logical_sector_size().await?);
            let total_size = self.ioctl(unsafe { rustix::ioctl::Getter::<BLKGETSIZE64, u64>::new() }).await?;
            match sector_size {
                0 => Err(Error::InvalidArgument),
                sector_size => Ok(total_size / sector_size),
            }
        }
    }
}

pub use ioctl::GenericIoctlDevice;
