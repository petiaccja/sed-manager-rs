use core::ffi::c_void;

use winapi::shared::ntddscsi::{
    IOCTL_SCSI_PASS_THROUGH_DIRECT, SCSI_IOCTL_DATA_IN, SCSI_IOCTL_DATA_OUT, SCSI_PASS_THROUGH_DIRECT,
};

use crate::device::shared::memory::write_nonoverlapping;
use crate::device::shared::scsi::{SecurityProtocolIn, SecurityProtocolOut};
use crate::device::windows::utility::{file_handle::FileHandle, ioctl::ioctl_in_out};
use crate::device::windows::Error as WindowsError;
use crate::device::{Device, Error as DeviceError, Interface};
use crate::serialization::SerializeBinary;

use super::GenericDevice;

pub struct SCSIDevice {
    generic_device: GenericDevice,
}

impl SCSIDevice {
    #[allow(unused)]
    pub fn open(path: &str) -> Result<Self, DeviceError> {
        // This does not check the interface, you can force SCSI on an unknown device.
        let generic_device = GenericDevice::open(path)?;
        Ok(Self { generic_device })
    }
}

impl TryFrom<GenericDevice> for SCSIDevice {
    type Error = GenericDevice;
    fn try_from(value: GenericDevice) -> Result<Self, Self::Error> {
        if let Ok(Interface::SCSI) = value.interface() {
            Ok(Self { generic_device: value })
        } else {
            Err(value)
        }
    }
}

impl Device for SCSIDevice {
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
        let protocol_specific = u16::from_be_bytes(protocol_specific);
        Ok(security_protocol_out(self.generic_device.get_file(), security_protocol, protocol_specific, data)?)
    }

    fn security_recv(
        &self,
        security_protocol: u8,
        protocol_specific: [u8; 2],
        len: usize,
    ) -> Result<Vec<u8>, DeviceError> {
        let mut data = vec![0; len];
        let protocol_specific = u16::from_be_bytes(protocol_specific);
        security_protocol_in(
            self.generic_device.get_file(),
            security_protocol,
            protocol_specific,
            data.as_mut_slice(),
        )?;
        Ok(data)
    }
}

pub fn security_protocol_in(
    file_handle: &FileHandle,
    security_protocol: u8,
    security_protocol_specific: u16,
    data_in: &mut [u8],
) -> Result<(), WindowsError> {
    let command = SecurityProtocolIn::new(security_protocol, security_protocol_specific, data_in.len() as u32);
    let cdb = command.to_bytes().expect("command serialization should be infallible");
    assert!(cdb.len() <= 16);
    let mut extended_cdb = cdb.iter().cloned().chain(core::iter::repeat(0));

    let command = SCSI_PASS_THROUGH_DIRECT {
        Length: size_of::<SCSI_PASS_THROUGH_DIRECT>() as u16,
        ScsiStatus: 0,
        PathId: 0,
        TargetId: 1,
        Lun: 0,
        CdbLength: cdb.len() as u8,
        SenseInfoLength: DEFAULT_SENSE_LENGTH,
        DataIn: SCSI_IOCTL_DATA_IN,
        DataTransferLength: data_in.len() as u32,
        TimeOutValue: 2,
        DataBuffer: data_in.as_mut_ptr() as *mut c_void,
        SenseInfoOffset: DEFAULT_SENSE_OFFSET,
        Cdb: core::array::from_fn(|_| extended_cdb.next().unwrap()),
    };

    let mut command_buffer = vec![0; command.SenseInfoOffset as usize + command.SenseInfoLength as usize];
    write_nonoverlapping(&command, &mut command_buffer);
    ioctl_in_out(file_handle.handle(), IOCTL_SCSI_PASS_THROUGH_DIRECT, &mut command_buffer).map(|_| ())
}

pub fn security_protocol_out(
    file_handle: &FileHandle,
    security_protocol: u8,
    security_protocol_specific: u16,
    data_out: &[u8],
) -> Result<(), WindowsError> {
    let command = SecurityProtocolOut::new(security_protocol, security_protocol_specific, data_out.len() as u32);
    let cdb = command.to_bytes().expect("command serialization should be infallible");
    assert!(cdb.len() <= 16);
    let mut extended_cdb = cdb.iter().cloned().chain(core::iter::repeat(0));

    let command = SCSI_PASS_THROUGH_DIRECT {
        Length: size_of::<SCSI_PASS_THROUGH_DIRECT>() as u16,
        ScsiStatus: 0,
        PathId: 0,
        TargetId: 1,
        Lun: 0,
        CdbLength: cdb.len() as u8,
        SenseInfoLength: DEFAULT_SENSE_LENGTH,
        DataIn: SCSI_IOCTL_DATA_OUT,
        DataTransferLength: data_out.len() as u32,
        TimeOutValue: 2,
        DataBuffer: data_out.as_ptr() as *mut c_void, // Data is not actually modified, hence the unsafe cast.
        SenseInfoOffset: DEFAULT_SENSE_OFFSET,
        Cdb: core::array::from_fn(|_| extended_cdb.next().unwrap()),
    };

    let mut command_buffer = vec![0; command.SenseInfoOffset as usize + command.SenseInfoLength as usize];
    write_nonoverlapping(&command, &mut command_buffer);
    ioctl_in_out(file_handle.handle(), IOCTL_SCSI_PASS_THROUGH_DIRECT, &mut command_buffer).map(|_| ())
}

const PTR_LENGTH: usize = size_of::<usize>();
const COMMAND_LENGTH: usize = size_of::<SCSI_PASS_THROUGH_DIRECT>();
const DEFAULT_SENSE_LENGTH: u8 = 32;
const DEFAULT_SENSE_OFFSET: u32 = ((COMMAND_LENGTH + PTR_LENGTH - 1) / PTR_LENGTH * PTR_LENGTH) as u32;
