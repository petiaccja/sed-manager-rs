use core::ptr::null_mut;

use nix::ioctl_readwrite;

use crate::device::linux::utility::FileHandle;
use crate::device::shared::nvme::{IdentifyController, Opcode};
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

    fn interface(&self) -> Result<Interface, Error> {
        Ok(Interface::NVMe)
    }

    fn model_number(&self) -> Result<String, Error> {
        Ok(String::from_utf8_lossy(&self.cached_desc.model_number).trim().to_string())
    }

    fn serial_number(&self) -> Result<String, Error> {
        Ok(String::from_utf8_lossy(&self.cached_desc.serial_number).trim().to_string())
    }

    fn firmware_revision(&self) -> Result<String, Error> {
        Ok(String::from_utf8_lossy(&self.cached_desc.firmware_revision).trim().to_string())
    }

    fn security_send(&self, security_protocol: u8, protocol_specific: [u8; 2], data: &[u8]) -> Result<(), Error> {
        if self.cached_desc.security_send_receive_supported {
            security_send(&self.file, security_protocol, protocol_specific, data)
        } else {
            Err(Error::NotSupported)
        }
    }

    fn security_recv(&self, security_protocol: u8, protocol_specific: [u8; 2], len: usize) -> Result<Vec<u8>, Error> {
        if self.cached_desc.security_send_receive_supported {
            let mut data = vec![0; len];
            security_receive(&self.file, security_protocol, protocol_specific, &mut data)?;
            Ok(data)
        } else {
            Err(Error::NotSupported)
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
    let _ = unsafe { nvme_admin_cmd(file.handle(), &mut command as *mut NVMeAdminCommand) }?;
    let identity = IdentifyController::from_bytes(identity).map_err(|_| Error::InterfaceMismatch)?;
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
    let _ = unsafe { nvme_admin_cmd(file_handle.handle(), &mut command as *mut NVMeAdminCommand) }?;
    Ok(())
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
    let _ = unsafe { nvme_admin_cmd(file_handle.handle(), &mut command as *mut NVMeAdminCommand) }?;
    Ok(())
}

fn make_cdw10(security_protocol: u8, protocol_specific: [u8; 2]) -> u32 {
    u32::from_be_bytes([
        security_protocol,
        protocol_specific[0],
        protocol_specific[1],
        0,
    ])
}

ioctl_readwrite!(nvme_admin_cmd, b'N', 0x41, NVMeAdminCommand);

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
