//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

//! Implements support for identify & security send/receive\* commands for ATA devices.
//!
//! \* Currently, only identify is implemented, so device information is displayed properly
//! but encryption is not actually supported. The official SCSI specs define
//! an ATA passthrough command. This appears to be exposed by Linux's `SG_IO` ioctl
//! and `ATA_12`/`ATA_16` SCSI opcodes. Support can be implemented with the `SG_IO`
//! ioctl and the `sg_io_hdr` structure. `hdparm`'s source code might be helpful.

use std::io;
use std::path::Path;

use sorbit::ser_de::FromBytes as _;

use crate::linux::ioctl_device::IoctlDevice;
use crate::shared::ata::{AtaError, IdentifyDevice};
use crate::{Error as DeviceError, Interface, StorageDevice};

pub use ioctl::AtaIoctlDevice;

pub struct AtaDevice {
    ioctl_device: IoctlDevice,
    desc: IdentifyDevice,
    is_removable: bool,
}

impl AtaDevice {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, DeviceError> {
        let ioctl_device = IoctlDevice::open(path).await?;
        let desc = ioctl_device.identify_device().await?;
        if desc.not_ata_device {
            return Err(DeviceError::InterfaceNotSupported);
        }
        let is_removable = query_is_removable(ioctl_device.path()).await.unwrap_or(false);
        Ok(Self { ioctl_device, desc, is_removable })
    }
}

#[async_trait::async_trait]
impl StorageDevice for AtaDevice {
    fn path(&self) -> Option<&Path> {
        Some(self.ioctl_device.path())
    }

    fn interface(&self) -> Interface {
        self.desc.interface()
    }

    fn model_number(&self) -> String {
        self.desc.model_number()
    }

    fn serial_number(&self) -> String {
        self.desc.serial_number()
    }

    fn firmware_revision(&self) -> String {
        self.desc.firmware_revision()
    }

    fn is_security_supported(&self) -> bool {
        self.desc.trusted_computing_supported
    }

    fn is_removable(&self) -> bool {
        self.is_removable
    }

    async fn security_send(
        &self,
        _security_protocol: u8,
        _protocol_specific: [u8; 2],
        _data: &[u8],
    ) -> Result<(), DeviceError> {
        if self.is_security_supported() {
            Err(DeviceError::NotImplemented)
        } else {
            Err(DeviceError::SecurityNotSupported)
        }
    }

    async fn security_recv(
        &self,
        _security_protocol: u8,
        _protocol_specific: [u8; 2],
        _len: usize,
    ) -> Result<Vec<u8>, DeviceError> {
        if self.is_security_supported() {
            Err(DeviceError::NotImplemented)
        } else {
            Err(DeviceError::SecurityNotSupported)
        }
    }
}

/// Reads the `removable` flag for a block device from sysfs, e.g. `/sys/block/sda/removable`.
async fn query_is_removable(path: impl AsRef<Path>) -> Result<bool, io::Error> {
    let path = path.as_ref().to_owned();
    blocking::unblock(move || {
        let file_name = path.file_name().ok_or(io::ErrorKind::InvalidFilename)?;
        let removable_path = Path::new("/sys/block").join(file_name).join("removable");
        std::fs::read_to_string(removable_path).map(|contents| contents.trim() == "1")
    })
    .await
}

mod ioctl {
    use super::*;

    /// `HDIO_GET_IDENTITY`. This is a raw legacy opcode, not one composed from
    /// `_IOR(group, num, size)`, so it must be used as a literal, not re-encoded
    /// via `rustix::ioctl::opcode::read`.
    const HDIO_GET_IDENTITY: rustix::ioctl::Opcode = 0x030d;

    pub trait AtaIoctlDevice {
        async fn identify_device(&self) -> Result<IdentifyDevice, DeviceError>;
    }

    impl AtaIoctlDevice for IoctlDevice {
        async fn identify_device(&self) -> Result<IdentifyDevice, DeviceError> {
            let identity = self.ioctl(unsafe { rustix::ioctl::Getter::<HDIO_GET_IDENTITY, [u8; 512]>::new() }).await?;
            IdentifyDevice::from_bytes(&identity).map_err(|_| DeviceError::ATAError(AtaError::with_error_bit()))
        }
    }
}
