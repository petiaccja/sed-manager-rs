use crate::device::device::{Device, DeviceError, Interface};
use crate::device::nvme::{NVMEIdentifyController, NVMeOpcode};
use crate::serialization::{Deserialize, InputStream};
use core::ptr::null_mut;
use std::cell::OnceCell;
use std::io::Seek;
use std::mem::{offset_of, zeroed};
use std::os::raw::c_void;
use std::os::windows::raw::HANDLE;
use winapi::shared::minwindef::DWORD;
use winapi::shared::ntddscsi::SCSI_PASS_THROUGH_DIRECT;
use winapi::shared::winerror::S_OK;
use winapi::um::fileapi::{CreateFileW, OPEN_EXISTING};
use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
use winapi::um::ioapiset::DeviceIoControl;
use winapi::um::winioctl::{IOCTL_STORAGE_QUERY_PROPERTY, STORAGE_PROPERTY_QUERY};
use winapi::um::winnt::{FILE_SHARE_READ, FILE_SHARE_WRITE, GENERIC_EXECUTE, GENERIC_READ, GENERIC_WRITE};

use super::error::get_last_error;
use super::string::string_to_wchars;

pub struct NVMeDevice {
    handle: HANDLE,
    cached_identity: OnceCell<NVMEIdentifyController>,
}

impl NVMeDevice {
    pub fn open(file_name: &str) -> Result<NVMeDevice, DeviceError> {
        let handle = nvme_open_device(file_name)?;
        Ok(NVMeDevice { handle: handle, cached_identity: OnceCell::new() })
    }
    fn identify_controller(&self) -> Result<&NVMEIdentifyController, DeviceError> {
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
    fn model_number(&self) -> Result<String, DeviceError> {
        Ok(self.identify_controller()?.model_number_as_str())
    }
    fn serial_number(&self) -> Result<String, DeviceError> {
        Ok(self.identify_controller()?.serial_number_as_str())
    }
    fn firmware_revision(&self) -> Result<String, DeviceError> {
        Ok(self.identify_controller()?.firmware_revision_as_str())
    }
    fn security_send(
        &self,
        security_protocol: u8,
        protocol_specific: [u8; 2],
        data: &[u8],
    ) -> Result<(), crate::device::device::DeviceError> {
        todo!()
    }
    fn security_recv(
        &self,
        security_protocol: u8,
        protocol_specific: [u8; 2],
        len: usize,
    ) -> Result<Vec<u8>, crate::device::device::DeviceError> {
        todo!()
    }
}

const NVME_MAX_LOG_SIZE: usize = 0x1000;

#[repr(u32)]
#[allow(nonstandard_style)]
#[allow(unused)]
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

fn nvme_open_device(file_name: &str) -> Result<HANDLE, DeviceError> {
    let mut file_name_utf16: Vec<u16> = string_to_wchars(file_name);
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
        if handle != INVALID_HANDLE_VALUE {
            Ok(handle)
        } else {
            Err(DeviceError::OSError(get_last_error()))
        }
    }
}

fn nvme_identify_controller(handle: HANDLE) -> Result<NVMEIdentifyController, DeviceError> {
    let mut buffer = vec![0u8; NVME_MAX_LOG_SIZE + 128];
    let data_offset = offset_of!(STORAGE_PROPERTY_QUERY, AdditionalParameters);
    let response_offset = size_of::<STORAGE_PROTOCOL_SPECIFIC_DATA>();

    let result = unsafe {
        let mut query: STORAGE_PROPERTY_QUERY = zeroed();
        query.PropertyId = 49; // StorageAdapterProtocolSpecificProperty
        query.QueryType = 0; // PropertyStandardQuery

        let mut data: STORAGE_PROTOCOL_SPECIFIC_DATA = zeroed();
        data.ProtocolType = STORAGE_PROTOCOL_TYPE::ProtocolTypeNvme;
        data.DataType = 1; // NVMeDataTypeIdentify;
        data.ProtocolDataRequestValue = 1; // NVME_IDENTIFY_CNS_CONTROLLER;
        data.ProtocolDataRequestSubValue = 0;
        data.ProtocolDataOffset = response_offset as u32;
        data.ProtocolDataLength = NVME_MAX_LOG_SIZE as u32;

        std::ptr::copy_nonoverlapping(
            &query as *const STORAGE_PROPERTY_QUERY,
            buffer.as_mut_ptr() as *mut STORAGE_PROPERTY_QUERY,
            data_offset,
        );
        std::ptr::copy_nonoverlapping(
            &data as *const STORAGE_PROTOCOL_SPECIFIC_DATA,
            buffer[data_offset..].as_mut_ptr() as *mut STORAGE_PROTOCOL_SPECIFIC_DATA,
            data_offset,
        );

        let mut bytes_returned: u32 = 0;
        DeviceIoControl(
            handle,
            IOCTL_STORAGE_QUERY_PROPERTY,
            buffer.as_mut_ptr() as *mut c_void,
            buffer.len() as u32,
            buffer.as_mut_ptr() as *mut c_void,
            buffer.len() as u32,
            &mut bytes_returned as *mut u32,
            null_mut(),
        )
    };

    if result == S_OK {
        Err(DeviceError::OSError(get_last_error()))
    } else {
        let mut stream = InputStream::from(buffer);
        stream.seek(std::io::SeekFrom::Start((data_offset + response_offset) as u64)).unwrap();
        match NVMEIdentifyController::deserialize(&mut stream) {
            Ok(i) => Ok(i),
            Err(_) => panic!("the stream must be long enough and the data is always deserializable"),
        }
    }
}

fn nvme_scsi_passthrough(opcode: NVMeOpcode,
    security_protocol : u8,
    protocol_specific: [u8; 2],
    sense_length: u8,
    sense_offset: u32,
    data: &[u8]) -> SCSI_PASS_THROUGH_DIRECT {
    SCSI_PASS_THROUGH_DIRECT {
        Length: size_of::<SCSI_PASS_THROUGH_DIRECT>() as u16,
        ScsiStatus: 0,
        PathId: 0,
        TargetId: 1,
        Lun: 0,
        CdbLength: todo!(),
        SenseInfoLength: todo!(),
        DataIn: todo!(),
        DataTransferLength: todo!(),
        TimeOutValue: todo!(),
        DataBuffer: todo!(),
        SenseInfoOffset: todo!(),
        Cdb: todo!(),
    }
}

#[cfg(test)]
mod test {
    use super::super::drive_list::{get_physical_drive_interface, get_physical_drives};
    use super::*;

    fn get_first_nvme_drive() -> Option<String> {
        if let Ok(drives) = get_physical_drives() {
            for drive in drives {
                if let Ok(interface) = get_physical_drive_interface(&drive) {
                    if interface == Interface::NVMe {
                        return Some(drive);
                    }
                }
            }
        }
        None
    }

    #[test]
    fn test_nvme_device_open() {
        if let Some(path) = get_first_nvme_drive() {
            match NVMeDevice::open(&path) {
                Ok(_) => (),
                Err(err) => panic!("failed to open device: {}", err),
            };
        };
    }

    #[test]
    fn test_nvme_identify_controller() {
        if let Some(path) = get_first_nvme_drive() {
            let device = match NVMeDevice::open(&path) {
                Ok(device) => device,
                Err(err) => panic!("failed to open device: {}", err),
            };
            match device.identify_controller() {
                Ok(identity) => identity,
                Err(err) => panic!("failed to identify controller: {}", err),
            };
        };
    }
}
