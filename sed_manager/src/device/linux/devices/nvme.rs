//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

//! Implements support for identify & security send/receive commands for NVMe devices.
//!
//! Uses native NVMe admin command ioctl's, i.e. no SCSI or other translation required.

use core::ptr::null_mut;

use nix::ioctl_readwrite;

use crate::device::linux::utility::FileHandle;
use crate::device::shared::nvme::{GenericStatusCode, IdentifyController, Opcode, StatusCode};
use crate::device::{Device, Error, Interface};
use crate::serialization::DeserializeBinary;

pub struct NVMeDevice {
    file: FileHandle,
    cached_desc: IdentifyController,
}

impl Device for NVMeDevice {
    fn path(&self) -> Option<String> {
        Some(self.file.path().into())
    }

    fn interface(&self) -> Interface {
        Interface::NVMe
    }

    fn model_number(&self) -> String {
        self.cached_desc.model_number_as_str()
    }

    fn serial_number(&self) -> String {
        self.cached_desc.serial_number_as_str()
    }

    fn firmware_revision(&self) -> String {
        self.cached_desc.firmware_revision_as_str()
    }

    fn is_security_supported(&self) -> bool {
        self.cached_desc.security_send_receive_supported
    }

    fn security_send(&self, security_protocol: u8, protocol_specific: [u8; 2], data: &[u8]) -> Result<(), Error> {
        if self.is_security_supported() {
            security_send(&self.file, security_protocol, protocol_specific, data)
        } else {
            Err(Error::SecurityNotSupported)
        }
    }

    fn security_recv(&self, security_protocol: u8, protocol_specific: [u8; 2], len: usize) -> Result<Vec<u8>, Error> {
        if self.is_security_supported() {
            let mut data = vec![0; len];
            security_receive(&self.file, security_protocol, protocol_specific, &mut data)?;
            Ok(data)
        } else {
            Err(Error::SecurityNotSupported)
        }
    }
}

impl NVMeDevice {
    pub fn open(path: &str) -> Result<Self, Error> {
        let file = FileHandle::open(path)?;
        let desc = identify_controller(&file)?;
        Ok(Self { file, cached_desc: desc })
    }
}

fn identify_controller(file: &FileHandle) -> Result<IdentifyController, Error> {
    let mut identity = vec![0_u8; 4096];
    let mut command = NVMeAdminCommand {
        opcode: Opcode::IdentifyController,
        addr: identity.as_mut_ptr(),
        data_len: identity.len() as u32,
        cdw10: 0x0000_0001,
        ..Default::default()
    };
    let ioctl_err = unsafe { nvme_admin_cmd(file.handle(), &mut command as *mut NVMeAdminCommand) }?;
    check_ioctl_err(ioctl_err)?;
    let identity = IdentifyController::from_bytes(identity).map_err(|_| Error::InterfaceNotSupported)?;
    Ok(identity)
}

fn security_receive(
    file_handle: &FileHandle,
    security_protocol: u8,
    protocol_specific: [u8; 2],
    data_in: &mut [u8],
) -> Result<(), Error> {
    let mut command = NVMeAdminCommand {
        opcode: Opcode::SecurityReceive,
        addr: data_in.as_mut_ptr(),
        data_len: data_in.len() as u32,
        cdw10: make_cdw10(security_protocol, protocol_specific),
        cdw11: data_in.len() as u32, // Data length duplicated.
        ..Default::default()
    };
    let ioctl_err = unsafe { nvme_admin_cmd(file_handle.handle(), &mut command as *mut NVMeAdminCommand) }?;
    check_ioctl_err(ioctl_err)
}

fn security_send(
    file_handle: &FileHandle,
    security_protocol: u8,
    protocol_specific: [u8; 2],
    data_out: &[u8],
) -> Result<(), Error> {
    let mut command = NVMeAdminCommand {
        opcode: Opcode::SecuritySend,
        addr: data_out.as_ptr() as *mut u8, // Data is not modified anyway.
        data_len: data_out.len() as u32,
        cdw10: make_cdw10(security_protocol, protocol_specific),
        cdw11: data_out.len() as u32, // Data length duplicated.
        ..Default::default()
    };
    let ioctl_err = unsafe { nvme_admin_cmd(file_handle.handle(), &mut command as *mut NVMeAdminCommand) }?;
    check_ioctl_err(ioctl_err)
}

fn make_cdw10(security_protocol: u8, protocol_specific: [u8; 2]) -> u32 {
    u32::from_be_bytes([
        security_protocol,
        protocol_specific[0],
        protocol_specific[1],
        0,
    ])
}

/// Check if the `ioctl` return value indicates an NVMe error.
/// The NVMe status is encoded in the lowest 11 bits of the value returned by `ioctl`.
fn check_ioctl_err(ioctl_err: i32) -> Result<(), Error> {
    let ioctl_err: u32 = unsafe { core::mem::transmute(ioctl_err) };
    let status = StatusCode::try_from(ioctl_err).unwrap_or(StatusCode::InvalidStatusField);
    match status {
        StatusCode::Generic(GenericStatusCode::Success) => match ioctl_err {
            0 => Ok(()),
            _ => Err(Error::NVMeError(StatusCode::Unknown(0))),
        },
        _ => Err(Error::NVMeError(status)),
    }
}

ioctl_readwrite!(nvme_admin_cmd, b'N', 0x41, NVMeAdminCommand);

#[derive(Debug)]
#[repr(C)]
struct NVMeAdminCommand {
    opcode: Opcode,
    flags: u8,
    rsvd1: u16,
    nsid: u32,
    cdw2: u32,
    cdw3: u32,
    metadata: *mut u8,
    addr: *mut u8,
    metadata_len: u32,
    data_len: u32,
    cdw10: u32,
    cdw11: u32,
    cdw12: u32,
    cdw13: u32,
    cdw14: u32,
    cdw15: u32,
    timeout_ms: u32,
    result: u32,
}

impl Default for NVMeAdminCommand {
    fn default() -> Self {
        Self {
            opcode: Opcode::IdentifyController,
            flags: 0,
            rsvd1: 0,
            nsid: 0,
            cdw2: 0,
            cdw3: 0,
            metadata: null_mut(),
            addr: null_mut(),
            metadata_len: 0,
            data_len: 0,
            cdw10: 0,
            cdw11: 0,
            cdw12: 0,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
            timeout_ms: 0,
            result: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_ioctl_err_success() {
        let ioctl_err = 0b000_00000000;
        assert_eq!(check_ioctl_err(ioctl_err), Ok(()));
    }

    #[test]
    fn check_ioctl_err_nvme_err() {
        let ioctl_err = 0b1_000_00000001;
        assert_eq!(
            check_ioctl_err(ioctl_err),
            Err(Error::NVMeError(StatusCode::Generic(GenericStatusCode::InvalidCommandOpcode)))
        );
    }

    #[test]
    fn check_ioctl_err_non_nvme_err() {
        let ioctl_err = 0b1_000_00000000;
        assert_eq!(check_ioctl_err(ioctl_err), Err(Error::NVMeError(StatusCode::Unknown(0))));
    }
}
