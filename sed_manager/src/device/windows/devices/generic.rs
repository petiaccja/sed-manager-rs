//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::ffi::c_void;
use core::ptr::null_mut;

use winapi::{
    shared::minwindef::DWORD,
    um::{
        ioapiset::DeviceIoControl,
        winioctl::{
            PropertyStandardQuery, StorageDeviceProperty, IOCTL_STORAGE_QUERY_PROPERTY, STORAGE_PROPERTY_QUERY,
        },
    },
};

use crate::device::windows::error::get_last_error;
use crate::device::windows::utility::file_handle::FileHandle;
use crate::device::windows::utility::ioctl::{STORAGE_BUS_TYPE, STORAGE_DEVICE_DESCRIPTOR};
use crate::device::{shared::string::FromNullTerminated, Device, Error, Interface};

pub struct GenericDevice {
    file: FileHandle,
    cached_desc: GenericDeviceDesc,
}

pub struct GenericDeviceDesc {
    pub interface: Interface,
    pub model_number: Option<String>,
    pub serial_number: Option<String>,
    pub firmware_revision: Option<String>,
}

impl Device for GenericDevice {
    fn path(&self) -> Option<String> {
        Some(self.file.path().into())
    }

    fn interface(&self) -> Interface {
        self.cached_desc.interface
    }

    fn model_number(&self) -> String {
        self.cached_desc.model_number.clone().unwrap_or(String::new())
    }

    fn serial_number(&self) -> String {
        self.cached_desc.serial_number.clone().unwrap_or(String::new())
    }

    fn firmware_revision(&self) -> String {
        self.cached_desc.firmware_revision.clone().unwrap_or(String::new())
    }

    fn is_security_supported(&self) -> bool {
        false
    }

    fn security_send(&self, _security_protocol: u8, _protocol_specific: [u8; 2], _data: &[u8]) -> Result<(), Error> {
        // The generic device does not support security commands.
        // This is because the IOCTL's may be interface-specific.
        Err(Error::NotImplemented)
    }

    fn security_recv(
        &self,
        _security_protocol: u8,
        _protocol_specific: [u8; 2],
        _len: usize,
    ) -> Result<Vec<u8>, Error> {
        // The generic device does not support security commands.
        Err(Error::NotImplemented)
    }
}

impl GenericDevice {
    pub fn open(path: &str) -> Result<Self, Error> {
        let file = FileHandle::open(path)?;
        let desc = query_description(&file)?;
        Ok(Self { file, cached_desc: desc })
    }

    pub fn get_file(&self) -> &FileHandle {
        &self.file
    }

    pub fn take_file(self) -> FileHandle {
        self.file
    }
}

impl GenericDeviceDesc {
    pub fn parse(descriptor: &STORAGE_DEVICE_DESCRIPTOR, buffer: &[u8]) -> Self {
        let interface = match descriptor.BusType {
            STORAGE_BUS_TYPE::BusTypeUnknown => Interface::Other,
            STORAGE_BUS_TYPE::BusTypeScsi => Interface::SCSI,
            STORAGE_BUS_TYPE::BusTypeAta => Interface::ATA,
            STORAGE_BUS_TYPE::BusTypeSata => Interface::SATA,
            STORAGE_BUS_TYPE::BusTypeSd => Interface::SD,
            STORAGE_BUS_TYPE::BusTypeMmc => Interface::MMC,
            STORAGE_BUS_TYPE::BusTypeNvme => Interface::NVMe,
            _ => Interface::Other,
        };
        let model_number = if descriptor.ProductIdOffset != 0 {
            let product_id_ptr = unsafe { buffer.as_ptr().add(descriptor.ProductIdOffset as usize) };
            String::from_null_terminated_utf8(product_id_ptr).map(|s| s.trim().into())
        } else {
            None
        };
        let serial_number = if descriptor.SerialNumberOffset != 0 {
            let product_id_ptr = unsafe { buffer.as_ptr().add(descriptor.SerialNumberOffset as usize) };
            String::from_null_terminated_utf8(product_id_ptr).map(|s| s.trim().into())
        } else {
            None
        };
        let firmware_revision = if descriptor.ProductRevisionOffset != 0 {
            let product_id_ptr = unsafe { buffer.as_ptr().add(descriptor.ProductRevisionOffset as usize) };
            String::from_null_terminated_utf8(product_id_ptr).map(|s| s.trim().into())
        } else {
            None
        };
        Self { interface, model_number, serial_number, firmware_revision }
    }
}

pub fn query_description(device: &FileHandle) -> Result<GenericDeviceDesc, Error> {
    match query_description_with_len(device, 2048)? {
        Ok(properties) => Ok(properties),
        Err(output_buffer_len) => {
            query_description_with_len(device, output_buffer_len)?.map_err(|_| Error::BufferTooShort)
        }
    }
}

fn query_description_with_len(
    device: &FileHandle,
    output_buffer_len: usize,
) -> Result<Result<GenericDeviceDesc, usize>, Error> {
    let mut query = STORAGE_PROPERTY_QUERY {
        PropertyId: StorageDeviceProperty,
        QueryType: PropertyStandardQuery,
        AdditionalParameters: [0],
    };

    let mut output_buffer = Vec::<u8>::new();
    output_buffer.resize(output_buffer_len, 0);
    let mut bytes_returned: DWORD = 0;

    let result = unsafe {
        DeviceIoControl(
            device.handle(),
            IOCTL_STORAGE_QUERY_PROPERTY,
            &mut query as *mut STORAGE_PROPERTY_QUERY as *mut c_void,
            size_of::<STORAGE_PROPERTY_QUERY>() as DWORD,
            output_buffer.as_mut_ptr() as *mut c_void,
            output_buffer.len() as DWORD,
            &mut bytes_returned as *mut DWORD,
            null_mut(),
        )
    };

    let descriptor = unsafe { &*(output_buffer.as_ptr() as *const STORAGE_DEVICE_DESCRIPTOR) };

    if result == 0 {
        get_last_error()?;
        Err(Error::Unspecified)
    } else if (descriptor.Size as usize) < output_buffer.len() {
        Ok(Ok(GenericDeviceDesc::parse(descriptor, &output_buffer)))
    } else {
        Ok(Err(descriptor.Size as usize))
    }
}
