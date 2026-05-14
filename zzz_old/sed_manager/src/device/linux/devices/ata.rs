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

use nix::ioctl_read_bad;

use crate::device::linux::utility::FileHandle;
use crate::device::shared::ata::IdentifyDevice;
use crate::device::{Device, Error as DeviceError, Interface};
use crate::serialization::DeserializeBinary;

pub struct ATADevice {
    file: FileHandle,
    cached_desc: IdentifyDevice,
}

impl Device for ATADevice {
    fn path(&self) -> Option<String> {
        Some(self.file.path().into())
    }

    fn interface(&self) -> Interface {
        self.cached_desc.interface()
    }

    fn model_number(&self) -> String {
        self.cached_desc.model_number()
    }

    fn serial_number(&self) -> String {
        self.cached_desc.serial_number()
    }

    fn firmware_revision(&self) -> String {
        self.cached_desc.firmware_revision()
    }

    fn is_security_supported(&self) -> bool {
        self.cached_desc.trusted_computing_supported
    }

    fn security_send(
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

    fn security_recv(
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

impl ATADevice {
    pub fn open(path: &str) -> Result<Self, DeviceError> {
        let file = FileHandle::open(path)?;
        let desc = query_description(&file)?;
        Ok(Self { file, cached_desc: desc })
    }
}

fn query_description(file: &FileHandle) -> Result<IdentifyDevice, DeviceError> {
    let mut identity = [0_u8; 512];
    let _ = unsafe { hdio_get_identity(file.handle(), &mut identity as *mut [u8; 512]) }?;
    let identity = IdentifyDevice::from_bytes(identity.into()).map_err(|_| DeviceError::InvalidArgument)?;
    if identity.not_ata_device {
        return Err(DeviceError::InterfaceNotSupported);
    }
    Ok(identity)
}

ioctl_read_bad!(hdio_get_identity, 0x030d, [u8; 512]);
