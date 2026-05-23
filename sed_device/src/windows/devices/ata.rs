//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::ffi::c_void;
use std::mem::transmute;
use std::path::Path;

use sorbit::ser_de::FromBytes;
use windows::Win32::Storage::IscsiDisc::{
    ATA_FLAGS_DATA_IN, ATA_FLAGS_DATA_OUT, ATA_FLAGS_USE_DMA, ATA_PASS_THROUGH_DIRECT, IOCTL_ATA_PASS_THROUGH_DIRECT,
};

use crate::shared::aligned_array::AlignedArray;
use crate::shared::ata::{AtaError, IdentifyDevice, Input};
use crate::windows::devices::generic::{GenericDeviceDesc, query_description};
use crate::windows::devices::raw_device::RawDevice;
use crate::{Device, Error as DeviceError, Interface};

use super::GenericDevice;

pub struct AtaDevice {
    raw_device: RawDevice,
    generic_desc: GenericDeviceDesc,
    desc: IdentifyDevice,
}

impl AtaDevice {
    #[allow(unused)]
    pub async fn open(path: &str) -> Result<Self, DeviceError> {
        let raw_device = RawDevice::open(path)?;
        let generic_desc = query_description(&raw_device).await?;
        let desc = identify_device(&raw_device).await?;
        Ok(Self { raw_device, generic_desc, desc })
    }

    pub async fn from_generic(value: GenericDevice) -> Result<Self, DeviceError> {
        if [Interface::ATA, Interface::SATA].contains(&value.interface()) {
            let raw_device = value.into_raw_device();
            let generic_desc = query_description(&raw_device).await?;
            let desc = identify_device(&raw_device).await?;
            Ok(Self { raw_device, generic_desc, desc })
        } else {
            Err(DeviceError::InterfaceNotSupported)
        }
    }
}

#[async_trait::async_trait]
impl Device for AtaDevice {
    fn path(&self) -> Option<&Path> {
        Some(self.raw_device.path())
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
        self.generic_desc.is_removable
    }

    async fn security_send(
        &self,
        security_protocol: u8,
        protocol_specific: [u8; 2],
        data: &[u8],
    ) -> Result<(), DeviceError> {
        if !self.is_security_supported() {
            return Err(DeviceError::SecurityNotSupported);
        }
        let aligned_data = AlignedArray::from_slice_padded(data, ALIGNMENT, PADDING).unwrap();
        let protocol_specific = u16::from_be_bytes(protocol_specific);
        Ok(
            trusted_send(&self.raw_device, security_protocol, protocol_specific, aligned_data.as_padded_slice())
                .await?,
        )
    }

    async fn security_recv(
        &self,
        security_protocol: u8,
        protocol_specific: [u8; 2],
        len: usize,
    ) -> Result<Vec<u8>, DeviceError> {
        if !self.is_security_supported() {
            return Err(DeviceError::SecurityNotSupported);
        }
        let mut data = AlignedArray::zeroed_padded(len, ALIGNMENT, PADDING).unwrap();
        let protocol_specific = u16::from_be_bytes(protocol_specific);
        trusted_receive(&self.raw_device, security_protocol, protocol_specific, data.as_padded_mut_slice()).await?;
        Ok(data.into_vec())
    }
}

async fn identify_device(file_handle: &RawDevice) -> Result<IdentifyDevice, DeviceError> {
    let mut data_out = vec![0_u8; 512];
    let input = Input::identify_device();
    let task_file = input.serialize();

    let command = ATA_PASS_THROUGH_DIRECT {
        Length: size_of::<ATA_PASS_THROUGH_DIRECT>() as u16,
        AtaFlags: (ATA_FLAGS_DATA_IN | ATA_FLAGS_USE_DMA) as u16,
        PathId: 0,          // Set by the driver.
        TargetId: 0,        // Set by the driver.
        Lun: 0,             // Set by the driver.
        ReservedAsUchar: 0, // Reserved for future use.
        DataTransferLength: data_out.len() as u32,
        TimeOutValue: TIMEOUT,
        ReservedAsUlong: 0, // Reserved for future use.
        DataBuffer: data_out.as_mut_ptr() as *mut c_void,
        PreviousTaskFile: [0; 8],
        CurrentTaskFile: task_file,
    };

    let mut request_buffer = make_request_buffer(command);
    let _ = file_handle
        .device_io_control_symmetric(IOCTL_ATA_PASS_THROUGH_DIRECT, request_buffer.as_mut_slice())
        .await?;
    parse_request_buffer(request_buffer)?;
    IdentifyDevice::from_bytes(&data_out).map_err(|_| DeviceError::ATAError(AtaError::with_error_bit()))
}

async fn trusted_send(
    file_handle: &RawDevice,
    security_protocol: u8,
    security_protocol_specific: u16,
    data_out: &[u8],
) -> Result<(), DeviceError> {
    let input = Input::trusted_send_dma(security_protocol, security_protocol_specific, data_out.len() as u32)?;
    let task_file = input.serialize();

    let command = ATA_PASS_THROUGH_DIRECT {
        Length: size_of::<ATA_PASS_THROUGH_DIRECT>() as u16,
        AtaFlags: (ATA_FLAGS_DATA_OUT | ATA_FLAGS_USE_DMA) as u16,
        PathId: 0,          // Set by the driver.
        TargetId: 0,        // Set by the driver.
        Lun: 0,             // Set by the driver.
        ReservedAsUchar: 0, // Reserved for future use.
        DataTransferLength: data_out.len() as u32,
        TimeOutValue: TIMEOUT,
        ReservedAsUlong: 0, // Reserved for future use.
        DataBuffer: data_out.as_ptr() as *mut c_void,
        PreviousTaskFile: [0; 8],
        CurrentTaskFile: task_file,
    };

    let mut request_buffer = make_request_buffer(command);
    let _ = file_handle
        .device_io_control_symmetric(IOCTL_ATA_PASS_THROUGH_DIRECT, request_buffer.as_mut_slice())
        .await?;
    parse_request_buffer(request_buffer).map_err(|err| err.into())
}

async fn trusted_receive(
    file_handle: &RawDevice,
    security_protocol: u8,
    security_protocol_specific: u16,
    data_out: &mut [u8],
) -> Result<(), DeviceError> {
    let input = Input::trusted_receive_dma(security_protocol, security_protocol_specific, data_out.len() as u32)?;
    let task_file = input.serialize();

    let command = ATA_PASS_THROUGH_DIRECT {
        Length: size_of::<ATA_PASS_THROUGH_DIRECT>() as u16,
        AtaFlags: (ATA_FLAGS_DATA_IN | ATA_FLAGS_USE_DMA) as u16,
        PathId: 0,          // Set by the driver.
        TargetId: 0,        // Set by the driver.
        Lun: 0,             // Set by the driver.
        ReservedAsUchar: 0, // Reserved for future use.
        DataTransferLength: data_out.len() as u32,
        TimeOutValue: TIMEOUT,
        ReservedAsUlong: 0, // Reserved for future use.
        DataBuffer: data_out.as_ptr() as *mut c_void,
        PreviousTaskFile: [0; 8],
        CurrentTaskFile: task_file,
    };

    let mut request_buffer = make_request_buffer(command);
    let _ = file_handle
        .device_io_control_symmetric(IOCTL_ATA_PASS_THROUGH_DIRECT, request_buffer.as_mut_slice())
        .await?;
    parse_request_buffer(request_buffer).map_err(|err| err.into())
}

/// See [`super::scsi`] for info about alignment.
const ALIGNMENT: usize = 8;

/// ATA trusted commands must have input and output buffers in 512 blocks.
const PADDING: usize = 512;

// Number of seconds to wait for the device to complete the ATA command.
const TIMEOUT: u32 = 10;

const REQUEST_BUFFER_LEN: usize = size_of::<ATA_PASS_THROUGH_DIRECT>();

fn make_request_buffer(command: ATA_PASS_THROUGH_DIRECT) -> [u8; REQUEST_BUFFER_LEN] {
    unsafe { transmute(command) }
}

fn parse_request_buffer(buffer: [u8; REQUEST_BUFFER_LEN]) -> Result<(), AtaError> {
    let command: ATA_PASS_THROUGH_DIRECT = unsafe { transmute(buffer) };
    let status = AtaError::from_task_file(command.CurrentTaskFile);
    if status.success() { Ok(()) } else { Err(status.into()) }
}
