use core::ffi::c_void;

use winapi::shared::ntddscsi::{
    ATA_FLAGS_DATA_IN, ATA_FLAGS_DATA_OUT, ATA_FLAGS_USE_DMA, ATA_PASS_THROUGH_DIRECT, IOCTL_ATA_PASS_THROUGH_DIRECT,
};

use crate::device::shared::aligned_array::AlignedArray;
use crate::device::shared::ata::{Input, Output};
use crate::device::windows::utility::file_handle::FileHandle;
use crate::device::windows::utility::ioctl::ioctl_in_out;
use crate::device::{Device, Error as DeviceError, Interface};

use super::GenericDevice;

pub struct ATADevice {
    generic_device: GenericDevice,
}

impl ATADevice {
    #[allow(unused)]
    pub fn open(path: &str) -> Result<Self, DeviceError> {
        // This does not check the interface, you can force SCSI on an unknown device.
        let generic_device = GenericDevice::open(path)?;
        Ok(Self { generic_device })
    }
}

impl TryFrom<GenericDevice> for ATADevice {
    type Error = GenericDevice;
    fn try_from(value: GenericDevice) -> Result<Self, Self::Error> {
        match value.interface() {
            Ok(Interface::ATA) => Ok(Self { generic_device: value }),
            Ok(Interface::SATA) => Ok(Self { generic_device: value }),
            _ => Err(value),
        }
    }
}

impl Device for ATADevice {
    fn path(&self) -> Option<String> {
        self.generic_device.path()
    }

    fn interface(&self) -> Result<Interface, DeviceError> {
        self.generic_device.interface()
    }

    fn model_number(&self) -> Result<String, DeviceError> {
        self.generic_device.model_number()
    }

    fn serial_number(&self) -> Result<String, DeviceError> {
        self.generic_device.serial_number()
    }

    fn firmware_revision(&self) -> Result<String, DeviceError> {
        self.generic_device.firmware_revision()
    }

    fn security_send(&self, security_protocol: u8, protocol_specific: [u8; 2], data: &[u8]) -> Result<(), DeviceError> {
        let aligned_data = AlignedArray::from_slice_padded(data, ALIGNMENT, PADDING).unwrap();
        let protocol_specific = u16::from_be_bytes(protocol_specific);
        Ok(trusted_send(
            self.generic_device.get_file(),
            security_protocol,
            protocol_specific,
            aligned_data.as_padded_slice(),
        )?)
    }

    fn security_recv(
        &self,
        security_protocol: u8,
        protocol_specific: [u8; 2],
        len: usize,
    ) -> Result<Vec<u8>, DeviceError> {
        let mut data = AlignedArray::zeroed_padded(len, ALIGNMENT, PADDING).unwrap();
        let protocol_specific = u16::from_be_bytes(protocol_specific);
        trusted_receive(
            self.generic_device.get_file(),
            security_protocol,
            protocol_specific,
            data.as_padded_mut_slice(),
        )?;
        Ok(data.into_vec())
    }
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
        TimeOutValue: 5,
        ReservedAsUlong: 0, // Reserved for future use.
        DataBuffer: data_out.as_ptr() as *mut c_void,
        PreviousTaskFile: [0; 8],
        CurrentTaskFile: task_file,
    };

    let command_buffer = unsafe {
        core::slice::from_raw_parts_mut(&mut command as *mut _ as *mut u8, size_of::<ATA_PASS_THROUGH_DIRECT>())
    };

    let _ = ioctl_in_out(file_handle.handle(), IOCTL_ATA_PASS_THROUGH_DIRECT, command_buffer)?;
    let output = Output::parse(command.CurrentTaskFile);
    if output.error || output.aborted || output.interface_crc {
        return Err(DeviceError::ATACommandAborted);
    }
    Ok(())
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
        TimeOutValue: 5,
        ReservedAsUlong: 0, // Reserved for future use.
        DataBuffer: data_out.as_ptr() as *mut c_void,
        PreviousTaskFile: [0; 8],
        CurrentTaskFile: task_file,
    };

    let command_buffer = unsafe {
        core::slice::from_raw_parts_mut(&mut command as *mut _ as *mut u8, size_of::<ATA_PASS_THROUGH_DIRECT>())
    };

    let _ = ioctl_in_out(file_handle.handle(), IOCTL_ATA_PASS_THROUGH_DIRECT, command_buffer)?;
    let output = Output::parse(command.CurrentTaskFile);
    if output.error || output.aborted || output.interface_crc {
        return Err(DeviceError::ATACommandAborted);
    }
    Ok(())
}

/// See [`super::scsi`] for info about alignment.
const ALIGNMENT: usize = 8;

/// ATA trusted commands must have input and output buffers in 512 blocks.
const PADDING: usize = 512;
