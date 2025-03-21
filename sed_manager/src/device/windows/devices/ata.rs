//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::ffi::c_void;

use winapi::shared::ntddscsi::{
    ATA_FLAGS_DATA_IN, ATA_FLAGS_DATA_OUT, ATA_FLAGS_USE_DMA, ATA_PASS_THROUGH_DIRECT, IOCTL_ATA_PASS_THROUGH_DIRECT,
};

use crate::device::shared::aligned_array::AlignedArray;
use crate::device::shared::ata::{ATAError, IdentifyDevice, Input};
use crate::device::windows::utility::file_handle::FileHandle;
use crate::device::windows::utility::ioctl::ioctl_in_out;
use crate::device::{Device, Error as DeviceError, Interface};
use crate::serialization::DeserializeBinary as _;

use super::GenericDevice;

pub struct ATADevice {
    file: FileHandle,
    cached_desc: IdentifyDevice,
}

impl ATADevice {
    #[allow(unused)]
    pub fn open(path: &str) -> Result<Self, DeviceError> {
        let file = FileHandle::open(path)?;
        let desc = identify_device(&file)?;
        Ok(Self { file, cached_desc: desc })
    }
}

impl TryFrom<GenericDevice> for ATADevice {
    type Error = DeviceError;
    fn try_from(value: GenericDevice) -> Result<Self, Self::Error> {
        if [Interface::ATA, Interface::SATA].contains(&value.interface()) {
            let desc = identify_device(value.get_file())?;
            Ok(Self { file: value.take_file(), cached_desc: desc })
        } else {
            Err(DeviceError::InterfaceNotSupported)
        }
    }
}

impl Device for ATADevice {
    fn path(&self) -> Option<String> {
        Some(self.file.path().to_string())
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

    fn security_send(&self, security_protocol: u8, protocol_specific: [u8; 2], data: &[u8]) -> Result<(), DeviceError> {
        if !self.is_security_supported() {
            return Err(DeviceError::SecurityNotSupported);
        }
        let aligned_data = AlignedArray::from_slice_padded(data, ALIGNMENT, PADDING).unwrap();
        let protocol_specific = u16::from_be_bytes(protocol_specific);
        Ok(trusted_send(&self.file, security_protocol, protocol_specific, aligned_data.as_padded_slice())?)
    }

    fn security_recv(
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
        trusted_receive(&self.file, security_protocol, protocol_specific, data.as_padded_mut_slice())?;
        Ok(data.into_vec())
    }
}

fn identify_device(file_handle: &FileHandle) -> Result<IdentifyDevice, DeviceError> {
    let mut data_out = vec![0_u8; 512];
    let input = Input::identify_device();
    let task_file = input.serialize();

    let mut command = ATA_PASS_THROUGH_DIRECT {
        Length: size_of::<ATA_PASS_THROUGH_DIRECT>() as u16,
        AtaFlags: ATA_FLAGS_DATA_IN | ATA_FLAGS_USE_DMA,
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

    let command_buffer = unsafe {
        core::slice::from_raw_parts_mut(&mut command as *mut _ as *mut u8, size_of::<ATA_PASS_THROUGH_DIRECT>())
    };

    let _ = ioctl_in_out(file_handle.handle(), IOCTL_ATA_PASS_THROUGH_DIRECT, command_buffer)?;
    check_ata_status(&command.CurrentTaskFile)?;
    IdentifyDevice::from_bytes(data_out).map_err(|_| DeviceError::ATAError(ATAError::with_error_bit()))
}

fn trusted_send(
    file_handle: &FileHandle,
    security_protocol: u8,
    security_protocol_specific: u16,
    data_out: &[u8],
) -> Result<(), DeviceError> {
    let input = Input::trusted_send_dma(security_protocol, security_protocol_specific, data_out.len() as u32)?;
    let task_file = input.serialize();

    let mut command = ATA_PASS_THROUGH_DIRECT {
        Length: size_of::<ATA_PASS_THROUGH_DIRECT>() as u16,
        AtaFlags: ATA_FLAGS_DATA_OUT | ATA_FLAGS_USE_DMA,
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

    let command_buffer = unsafe {
        core::slice::from_raw_parts_mut(&mut command as *mut _ as *mut u8, size_of::<ATA_PASS_THROUGH_DIRECT>())
    };

    let _ = ioctl_in_out(file_handle.handle(), IOCTL_ATA_PASS_THROUGH_DIRECT, command_buffer)?;
    check_ata_status(&command.CurrentTaskFile)
}

fn trusted_receive(
    file_handle: &FileHandle,
    security_protocol: u8,
    security_protocol_specific: u16,
    data_out: &mut [u8],
) -> Result<(), DeviceError> {
    let input = Input::trusted_receive_dma(security_protocol, security_protocol_specific, data_out.len() as u32)?;
    let task_file = input.serialize();

    let mut command = ATA_PASS_THROUGH_DIRECT {
        Length: size_of::<ATA_PASS_THROUGH_DIRECT>() as u16,
        AtaFlags: ATA_FLAGS_DATA_IN | ATA_FLAGS_USE_DMA,
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

    let command_buffer = unsafe {
        core::slice::from_raw_parts_mut(&mut command as *mut _ as *mut u8, size_of::<ATA_PASS_THROUGH_DIRECT>())
    };

    let _ = ioctl_in_out(file_handle.handle(), IOCTL_ATA_PASS_THROUGH_DIRECT, command_buffer)?;
    check_ata_status(&command.CurrentTaskFile)
}

/// See [`super::scsi`] for info about alignment.
const ALIGNMENT: usize = 8;

/// ATA trusted commands must have input and output buffers in 512 blocks.
const PADDING: usize = 512;

// Number of seconds to wait for the device to complete the ATA command.
const TIMEOUT: u32 = 10;

fn check_ata_status(task_file: &[u8; 8]) -> Result<(), DeviceError> {
    let status = ATAError::from_task_file(task_file.clone());
    if status.success() {
        Ok(())
    } else {
        Err(DeviceError::ATAError(status))
    }
}
