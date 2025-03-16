//! Implements support for identify & security send/receive\* commands for ATA devices.
//!
//! \* Currently, only identify is implemented, so device information is displayed properly
//! but encryption is not actually supported. The official SCSI specs define
//! an ATA passthrough command. This appears to be exposed by Linux's `SG_IO` ioctl
//! and `ATA_12`/`ATA_16` SCSI opcodes. Support can be implemented with the `SG_IO`
//! ioctl and the `sg_io_hdr` structure. `hdparm`'s source code might be helpful.

use nix::errno::Errno;
use nix::libc::ioctl;

use crate::device::linux::utility::FileHandle;
use crate::device::linux::Error as LinuxError;
use crate::device::shared::ata::IdentifyDevice;
use crate::device::{Device, Error, Interface};
use crate::serialization::DeserializeBinary;

pub struct ATADevice {
    file: FileHandle,
    cached_desc: IdentifyDevice,
}

impl Device for ATADevice {
    fn path(&self) -> Option<String> {
        Some(self.file.path().into())
    }

    fn interface(&self) -> Result<Interface, DeviceError> {
        Ok(self.cached_desc.interface())
    }

    fn model_number(&self) -> Result<String, DeviceError> {
        Ok(self.cached_desc.model_number())
    }

    fn serial_number(&self) -> Result<String, DeviceError> {
        Ok(self.cached_desc.serial_number())
    }

    fn firmware_revision(&self) -> Result<String, DeviceError> {
        Ok(self.cached_desc.firmware_revision())
    }

    fn is_security_supported(&self) -> bool {
        self.cached_desc.trusted_computing_supported
    }

    fn security_send(&self, _security_protocol: u8, _protocol_specific: [u8; 2], _data: &[u8]) -> Result<(), Error> {
        if self.is_security_supported() {
            Err(Error::NotImplemented)
        } else {
            Err(Error::SecurityNotSupported)
        }
    }

    fn security_recv(
        &self,
        _security_protocol: u8,
        _protocol_specific: [u8; 2],
        _len: usize,
    ) -> Result<Vec<u8>, Error> {
        if self.is_security_supported() {
            Err(Error::NotImplemented)
        } else {
            Err(Error::SecurityNotSupported)
        }
    }
}

impl ATADevice {
    pub fn open(path: &str) -> Result<Self, Error> {
        let file = FileHandle::open(path)?;
        let desc = query_description(&file)?;
        Ok(Self { file, cached_desc: desc })
    }
}

fn query_description(file: &FileHandle) -> Result<IdentifyDevice, Error> {
    let mut identity = [0_u8; 512];
    let result = unsafe { ioctl(file.handle(), HDIO_GET_IDENTITY, identity.as_mut_ptr()) };
    if result != 0 {
        return Err(LinuxError::from(Errno::from_raw(result)).into());
    }
    let identity = IdentifyDevice::from_bytes(identity.into()).map_err(|_| Error::InvalidArgument)?;
    if identity.not_ata_device {
        return Err(Error::InterfaceMismatch);
    }
    Ok(identity)
}

const HDIO_GET_IDENTITY: u64 = 0x030d; // IO control code [include/uapi/linux/hdreg.h]
