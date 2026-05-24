//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::ffi::CStr;
use std::mem::transmute;
use std::path::Path;

use windows::Win32::Storage::FileSystem::*;
use windows::Win32::System::Ioctl::{
    IOCTL_STORAGE_QUERY_PROPERTY, PropertyStandardQuery, STORAGE_DEVICE_DESCRIPTOR, STORAGE_PROPERTY_QUERY,
    StorageDeviceProperty,
};

use crate::windows::devices::raw_device::RawDevice;
use crate::{Device, Error, Interface};

pub struct GenericDevice {
    raw_device: RawDevice,
    cached_desc: GenericDeviceDesc,
}

pub struct GenericDeviceDesc {
    pub interface: Interface,
    pub model_number: Option<String>,
    pub serial_number: Option<String>,
    pub firmware_revision: Option<String>,
    pub is_removable: bool,
}

#[async_trait::async_trait]
impl Device for GenericDevice {
    fn path(&self) -> Option<&Path> {
        Some(&self.raw_device.path())
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

    fn is_removable(&self) -> bool {
        false
    }

    async fn security_send(
        &self,
        _security_protocol: u8,
        _protocol_specific: [u8; 2],
        _data: &[u8],
    ) -> Result<(), Error> {
        // The generic device does not support security commands.
        // This is because the IOCTL's may be interface-specific.
        Err(Error::NotImplemented)
    }

    async fn security_recv(
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
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        let file = RawDevice::open(path).await?;
        let desc = query_description(&file).await?;
        Ok(Self { raw_device: file, cached_desc: desc })
    }

    pub fn get_file(&self) -> &RawDevice {
        &self.raw_device
    }

    pub fn into_raw_device(self) -> RawDevice {
        self.raw_device
    }
}

impl GenericDeviceDesc {
    pub fn parse(descriptor: &STORAGE_DEVICE_DESCRIPTOR, buffer: &[u8]) -> Self {
        #[allow(non_upper_case_globals)]
        let interface = match descriptor.BusType {
            BusTypeUnknown => Interface::Other,
            BusTypeScsi => Interface::SCSI,
            BusTypeAta => Interface::ATA,
            BusTypeSata => Interface::SATA,
            BusTypeSd => Interface::SD,
            BusTypeMmc => Interface::MMC,
            BusTypeNvme => Interface::NVMe,
            BusTypeUsb => Interface::USB,
            _ => Interface::Other,
        };
        let model_number = if descriptor.ProductIdOffset != 0 {
            let ptr = unsafe { buffer.as_ptr().add(descriptor.ProductIdOffset as usize) };
            let cstr = unsafe { CStr::from_ptr(ptr as *const i8) };
            Some(cstr.to_string_lossy().trim().to_owned())
        } else {
            None
        };
        let serial_number = if descriptor.SerialNumberOffset != 0 {
            let ptr = unsafe { buffer.as_ptr().add(descriptor.SerialNumberOffset as usize) };
            let cstr = unsafe { CStr::from_ptr(ptr as *const i8) };
            Some(cstr.to_string_lossy().trim().to_owned())
        } else {
            None
        };
        let firmware_revision = if descriptor.ProductRevisionOffset != 0 {
            let ptr = unsafe { buffer.as_ptr().add(descriptor.ProductRevisionOffset as usize) };
            let cstr = unsafe { CStr::from_ptr(ptr as *const i8) };
            Some(cstr.to_string_lossy().trim().to_owned())
        } else {
            None
        };
        let is_removable = descriptor.RemovableMedia;
        Self { interface, model_number, serial_number, firmware_revision, is_removable }
    }
}

pub async fn query_description(device: &RawDevice) -> Result<GenericDeviceDesc, Error> {
    match query_description_with_len(device, 2048).await? {
        Ok(properties) => Ok(properties),
        Err(output_buffer_len) => {
            query_description_with_len(device, output_buffer_len).await?.map_err(|_| Error::BufferTooShort)
        }
    }
}

async fn query_description_with_len(
    device: &RawDevice,
    response_buffer_len: usize,
) -> Result<Result<GenericDeviceDesc, usize>, Error> {
    let request = STORAGE_PROPERTY_QUERY {
        PropertyId: StorageDeviceProperty,
        QueryType: PropertyStandardQuery,
        AdditionalParameters: [0],
    };

    let mut request_buffer: [u8; size_of::<STORAGE_PROPERTY_QUERY>()] = unsafe { transmute(request) };
    let mut response_buffer = vec![0u8; response_buffer_len];

    device
        .device_io_control(IOCTL_STORAGE_QUERY_PROPERTY, Some(&mut request_buffer), Some(&mut response_buffer))
        .await?;

    let descriptor = unsafe { &*(response_buffer.as_ptr() as *const STORAGE_DEVICE_DESCRIPTOR) };
    if (descriptor.Size as usize) < response_buffer.len() {
        Ok(Ok(GenericDeviceDesc::parse(descriptor, &response_buffer)))
    } else {
        Ok(Err(descriptor.Size as usize))
    }
}
