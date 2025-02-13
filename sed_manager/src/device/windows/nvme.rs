use super::error::Error;
use crate::device;
use crate::device::device::{Device, Interface};
use crate::device::nvme::{NVMeIdentifyController, NVMeOpcode};
use crate::device::shared::string::ToNullTerminated;
use crate::serialization::{Deserialize, InputStream};

use core::ptr::null_mut;
use std::io::Seek;
use std::mem::offset_of;
use std::os::raw::c_void;
use std::os::windows::raw::HANDLE;
use std::sync::OnceLock;
use winapi::shared::minwindef::{DWORD, FALSE};
use winapi::shared::ntddscsi::{
    IOCTL_SCSI_PASS_THROUGH_DIRECT, SCSI_IOCTL_DATA_IN, SCSI_IOCTL_DATA_OUT, SCSI_PASS_THROUGH_DIRECT,
};
use winapi::um::fileapi::{CreateFileW, OPEN_EXISTING};
use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
use winapi::um::ioapiset::DeviceIoControl;
use winapi::um::winioctl::{IOCTL_STORAGE_QUERY_PROPERTY, STORAGE_PROPERTY_QUERY};
use winapi::um::winnt::{FILE_SHARE_READ, FILE_SHARE_WRITE, GENERIC_EXECUTE, GENERIC_READ, GENERIC_WRITE};

use super::error::get_last_error;

pub struct NVMeDevice {
    handle: HANDLE,
    cached_identity: OnceLock<NVMeIdentifyController>,
}

unsafe impl Send for NVMeDevice {}
unsafe impl Sync for NVMeDevice {}

impl NVMeDevice {
    pub fn open(file_name: &str) -> Result<NVMeDevice, device::Error> {
        let handle = nvme_open_device(file_name)?;
        Ok(NVMeDevice { handle: handle, cached_identity: OnceLock::new() })
    }
    fn identify_controller(&self) -> Result<&NVMeIdentifyController, device::Error> {
        match self.cached_identity.get() {
            Some(identity) => Ok(identity),
            None => {
                let identity = nvme_identify_controller(self.handle)?;
                Ok(self.cached_identity.get_or_init(|| identity))
            }
        }
    }
}

impl Drop for NVMeDevice {
    fn drop(&mut self) {
        assert!(self.handle != INVALID_HANDLE_VALUE);
        unsafe {
            CloseHandle(self.handle);
        };
    }
}

impl Device for NVMeDevice {
    fn interface(&self) -> Interface {
        Interface::NVMe
    }
    fn model_number(&self) -> Result<String, device::Error> {
        Ok(self.identify_controller()?.model_number_as_str().trim().to_string())
    }
    fn serial_number(&self) -> Result<String, device::Error> {
        Ok(self.identify_controller()?.serial_number_as_str().trim().to_string())
    }
    fn firmware_revision(&self) -> Result<String, device::Error> {
        Ok(self.identify_controller()?.firmware_revision_as_str().trim().to_string())
    }
    fn security_send(
        &self,
        security_protocol: u8,
        protocol_specific: [u8; 2],
        data: &[u8],
    ) -> Result<(), device::Error> {
        // This is kind of annoying, because security send will NOT modify this data.
        // Still have to copy it to make it mutable for the borrow checker.
        let mut copy = Vec::from(data);
        let opcode = NVMeOpcode::SecuritySend;
        Ok(nvme_security_command(self.handle, opcode, security_protocol, protocol_specific, &mut copy)?)
    }
    fn security_recv(
        &self,
        security_protocol: u8,
        protocol_specific: [u8; 2],
        len: usize,
    ) -> Result<Vec<u8>, device::Error> {
        let mut data = vec![0_u8; len];
        let opcode = NVMeOpcode::SecurityReceive;
        nvme_security_command(self.handle, opcode, security_protocol, protocol_specific, &mut data)?;
        Ok(data)
    }
}

const NVME_MAX_LOG_SIZE: usize = 0x1000;

#[repr(u32)]
#[allow(nonstandard_style)]
#[allow(unused)]
#[derive(Debug)]
enum STORAGE_PROTOCOL_TYPE {
    ProtocolTypeUnknown = 0x00,
    ProtocolTypeScsi,
    ProtocolTypeAta,
    ProtocolTypeNvme,
    ProtocolTypeSd,
    ProtocolTypeUfs,
    ProtocolTypeProprietary = 0x7E,
    ProtocolTypeMaxReserved = 0x7F,
}

#[repr(C)]
#[allow(nonstandard_style)]
#[allow(unused)]
struct STORAGE_PROTOCOL_SPECIFIC_DATA {
    ProtocolType: STORAGE_PROTOCOL_TYPE,
    DataType: DWORD,
    ProtocolDataRequestValue: DWORD,
    ProtocolDataRequestSubValue: DWORD,
    ProtocolDataOffset: DWORD,
    ProtocolDataLength: DWORD,
    FixedProtocolReturnData: DWORD,
    ProtocolDataRequestSubValue2: DWORD,
    ProtocolDataRequestSubValue3: DWORD,
    ProtocolDataRequestSubValue4: DWORD,
}

fn nvme_open_device(file_name: &str) -> Result<HANDLE, Error> {
    let mut file_name_utf16: Vec<u16> = file_name.to_null_terminated_utf16();
    unsafe {
        let handle = CreateFileW(
            file_name_utf16.as_mut_ptr(),
            GENERIC_READ | GENERIC_WRITE | GENERIC_EXECUTE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            null_mut(),
            OPEN_EXISTING,
            0,
            null_mut(),
        );
        if handle == INVALID_HANDLE_VALUE {
            get_last_error()?;
        };
        Ok(handle)
    }
}

fn nvme_device_io_control(handle: HANDLE, ioctl: DWORD, buffer: &mut [u8]) -> Result<u32, Error> {
    let mut bytes_returned: u32 = 0;
    let result = unsafe {
        DeviceIoControl(
            handle,
            ioctl,
            buffer.as_mut_ptr() as *mut c_void,
            buffer.len() as u32,
            buffer.as_mut_ptr() as *mut c_void,
            buffer.len() as u32,
            &mut bytes_returned as *mut u32,
            null_mut(),
        )
    };

    if result == FALSE {
        get_last_error()?;
    };
    Ok(bytes_returned)
}

unsafe fn serialize_raw<T>(value: &T, dst: &mut [u8]) {
    std::ptr::copy_nonoverlapping(value as *const T, dst.as_mut_ptr() as *mut T, 1);
}

// This is useful for debugging, but may not be actually used in the code.
#[allow(unused)]
unsafe fn deserialize_raw<T>(src: &[u8], value: &mut T) {
    std::ptr::copy_nonoverlapping(src.as_ptr() as *const T, value as *mut T, 1);
}

fn nvme_identify_controller(handle: HANDLE) -> Result<NVMeIdentifyController, Error> {
    let mut buffer = vec![0u8; NVME_MAX_LOG_SIZE + 128];
    let data_offset = offset_of!(STORAGE_PROPERTY_QUERY, AdditionalParameters);
    let response_offset = size_of::<STORAGE_PROTOCOL_SPECIFIC_DATA>();

    let query = STORAGE_PROPERTY_QUERY {
        PropertyId: 49, // StorageAdapterProtocolSpecificProperty
        QueryType: 0,   // PropertyStandardQuery
        AdditionalParameters: [0],
    };

    let data = STORAGE_PROTOCOL_SPECIFIC_DATA {
        ProtocolType: STORAGE_PROTOCOL_TYPE::ProtocolTypeNvme,
        DataType: 1,                 // NVMeDataTypeIdentify
        ProtocolDataRequestValue: 1, // NVME_IDENTIFY_CNS_CONTROLLER
        ProtocolDataRequestSubValue: 0,
        ProtocolDataOffset: response_offset as u32,
        ProtocolDataLength: NVME_MAX_LOG_SIZE as u32,
        FixedProtocolReturnData: 0,
        ProtocolDataRequestSubValue2: 0,
        ProtocolDataRequestSubValue3: 0,
        ProtocolDataRequestSubValue4: 0,
    };

    unsafe { serialize_raw(&query, &mut buffer) };
    unsafe { serialize_raw(&data, &mut buffer[data_offset..]) };

    let _ = nvme_device_io_control(handle, IOCTL_STORAGE_QUERY_PROPERTY, &mut buffer)?;

    let mut stream = InputStream::from(buffer);
    stream.seek(std::io::SeekFrom::Start((data_offset + response_offset) as u64)).unwrap();
    match NVMeIdentifyController::deserialize(&mut stream) {
        Ok(identity) => Ok(identity),
        Err(_) => panic!("controller identify structure deserialization should not fail"),
    }
}

fn nvme_scsi_passthrough(
    opcode: NVMeOpcode,
    security_protocol: u8,
    protocol_specific: [u8; 2],
    sense_length: u8,
    sense_offset: u32,
    data: &mut [u8],
) -> SCSI_PASS_THROUGH_DIRECT {
    let cdb: [u8; 16] = [
        match opcode {
            NVMeOpcode::SecuritySend => 0xB5,
            NVMeOpcode::SecurityReceive => 0xA2,
            _ => panic!("opcode not supported with SCSI passthrough"),
        },
        security_protocol,
        protocol_specific[0],
        protocol_specific[1],
        0,
        0,
        ((data.len() >> 24) & 0xFF) as u8,
        ((data.len() >> 16) & 0xFF) as u8,
        ((data.len() >> 8) & 0xFF) as u8,
        ((data.len()) & 0xFF) as u8,
        0,
        0,
        0,
        0,
        0,
        0,
    ];
    SCSI_PASS_THROUGH_DIRECT {
        Length: size_of::<SCSI_PASS_THROUGH_DIRECT>() as u16,
        ScsiStatus: 0,
        PathId: 0,
        TargetId: 1,
        Lun: 0,
        CdbLength: 12,
        SenseInfoLength: sense_length,
        DataIn: match opcode {
            NVMeOpcode::SecuritySend => SCSI_IOCTL_DATA_OUT,
            NVMeOpcode::SecurityReceive => SCSI_IOCTL_DATA_IN,
            _ => panic!("opcode not supported with SCSI passthrough"),
        },
        DataTransferLength: data.len() as u32,
        TimeOutValue: 2,
        DataBuffer: data.as_mut_ptr() as *mut c_void,
        SenseInfoOffset: sense_offset,
        Cdb: cdb,
    }
}

fn nvme_security_command(
    handle: HANDLE,
    opcode: NVMeOpcode,
    security_protocol: u8,
    protocol_specific: [u8; 2],
    data: &mut [u8],
) -> Result<(), Error> {
    const PTR_LENGTH: usize = size_of::<usize>();
    const COMMAND_LENGTH: usize = size_of::<SCSI_PASS_THROUGH_DIRECT>();
    const SENSE_LENGTH: usize = 32;
    const SENSE_OFFSET: usize = (COMMAND_LENGTH + PTR_LENGTH - 1) / PTR_LENGTH * PTR_LENGTH;

    let mut command_buffer = [0u8; SENSE_OFFSET + SENSE_LENGTH];

    let command = nvme_scsi_passthrough(
        opcode,
        security_protocol,
        protocol_specific,
        SENSE_LENGTH as u8,
        SENSE_OFFSET as u32,
        data,
    );
    unsafe { serialize_raw(&command, &mut command_buffer) };
    match nvme_device_io_control(handle, IOCTL_SCSI_PASS_THROUGH_DIRECT, &mut command_buffer) {
        Ok(_) => Ok(()),
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod test {
    use super::super::drive_list::{get_drive_interface, list_physical_drives};
    use super::*;
    use skip_test::{may_skip, skip, skip_or_unwrap};

    fn list_nvme_drives() -> Vec<String> {
        let drives = list_physical_drives().ok().unwrap_or(vec![]);
        let nvme_drives: Vec<_> = drives
            .into_iter()
            .filter(move |drive| -> bool { get_drive_interface(drive).unwrap_or(Interface::Other) == Interface::NVMe })
            .collect();
        nvme_drives
    }

    #[test]
    #[may_skip]
    fn nvme_device_open_any() -> Result<(), device::Error> {
        let nvme_drives = list_nvme_drives();
        let path = skip_or_unwrap!(nvme_drives.first());
        let device = NVMeDevice::open(path);
        match device {
            Ok(_) => Ok(()),
            Err(device::Error::PermissionDenied) => skip!(),
            Err(err) => Err(err),
        }
    }

    #[test]
    #[may_skip]
    fn test_nvme_identify_controller() -> Result<(), device::Error> {
        let nvme_drives = list_nvme_drives();
        let path = skip_or_unwrap!(nvme_drives.first());
        let device = skip_or_unwrap!(NVMeDevice::open(path));
        match device.identify_controller() {
            Ok(_) => Ok(()),
            Err(device::Error::PermissionDenied) => skip!(),
            Err(err) => Err(err.into()),
        }
    }
}
