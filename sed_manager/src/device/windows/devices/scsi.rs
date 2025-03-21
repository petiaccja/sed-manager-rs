//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::ffi::c_void;

use winapi::shared::ntddscsi::{
    IOCTL_SCSI_PASS_THROUGH_DIRECT, SCSI_IOCTL_DATA_IN, SCSI_IOCTL_DATA_OUT, SCSI_PASS_THROUGH_DIRECT,
};

use crate::device::shared::aligned_array::AlignedArray;
use crate::device::shared::scsi::{
    DescriptorSenseData, FixedSenseData, SCSIError, SecurityProtocolIn, SecurityProtocolOut, SenseKey,
    SenseResponseCode,
};
use crate::device::windows::utility::{file_handle::FileHandle, ioctl::ioctl_in_out};
use crate::device::{Device, Error as DeviceError, Interface};
use crate::serialization::{DeserializeBinary, SerializeBinary};

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
    type Error = DeviceError;
    fn try_from(value: GenericDevice) -> Result<Self, Self::Error> {
        if let Interface::SCSI = value.interface() {
            Ok(Self { generic_device: value })
        } else {
            Err(DeviceError::InterfaceNotSupported)
        }
    }
}

impl Device for SCSIDevice {
    fn path(&self) -> Option<String> {
        self.generic_device.path()
    }

    fn interface(&self) -> Interface {
        self.generic_device.interface()
    }

    fn model_number(&self) -> String {
        self.generic_device.model_number()
    }

    fn serial_number(&self) -> String {
        self.generic_device.serial_number()
    }

    fn firmware_revision(&self) -> String {
        self.generic_device.firmware_revision()
    }

    fn is_security_supported(&self) -> bool {
        // Getting the SCSI identification is way to complicated.
        // We'll just assume `true` and let the security send/receive commands fail
        // if the device does not actually support them.
        true
    }

    fn security_send(&self, security_protocol: u8, protocol_specific: [u8; 2], data: &[u8]) -> Result<(), DeviceError> {
        let aligned_data = AlignedArray::from_slice_padded(data, ALIGNMENT, PADDING).unwrap();
        let protocol_specific = u16::from_be_bytes(protocol_specific);
        Ok(security_protocol_out(
            self.generic_device.get_file(),
            security_protocol,
            protocol_specific,
            aligned_data.as_padded_slice(),
            get_inc_512_flag(security_protocol),
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
        security_protocol_in(
            self.generic_device.get_file(),
            security_protocol,
            protocol_specific,
            data.as_padded_mut_slice(),
            get_inc_512_flag(security_protocol),
        )?;
        Ok(data.into_vec())
    }
}

pub fn security_protocol_in(
    file_handle: &FileHandle,
    security_protocol: u8,
    security_protocol_specific: u16,
    data_in: &mut [u8],
    inc_512: bool,
) -> Result<(), DeviceError> {
    let command = SecurityProtocolIn::new(security_protocol, security_protocol_specific, data_in.len() as u32, inc_512);
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
        SenseInfoLength: 0,
        DataIn: SCSI_IOCTL_DATA_IN,
        DataTransferLength: data_in.len() as u32,
        TimeOutValue: 2,
        DataBuffer: data_in.as_mut_ptr() as *mut c_void,
        SenseInfoOffset: 0,
        Cdb: core::array::from_fn(|_| extended_cdb.next().unwrap()),
    };

    let mut command_buffer = CommandWithSense::new(command);
    let _ = ioctl_in_out(file_handle.handle(), IOCTL_SCSI_PASS_THROUGH_DIRECT, command_buffer.as_mut_slice())?;
    check_sense_info(command_buffer.command.ScsiStatus, &command_buffer.sense_info)
}

pub fn security_protocol_out(
    file_handle: &FileHandle,
    security_protocol: u8,
    security_protocol_specific: u16,
    data_out: &[u8],
    inc_512: bool,
) -> Result<(), DeviceError> {
    let command =
        SecurityProtocolOut::new(security_protocol, security_protocol_specific, data_out.len() as u32, inc_512);
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
        SenseInfoLength: 0,
        DataIn: SCSI_IOCTL_DATA_OUT,
        DataTransferLength: data_out.len() as u32,
        TimeOutValue: 2,
        DataBuffer: data_out.as_ptr() as *mut c_void, // Data is not actually modified, hence the unsafe cast.
        SenseInfoOffset: 0,
        Cdb: core::array::from_fn(|_| extended_cdb.next().unwrap()),
    };

    let mut command_buffer = CommandWithSense::new(command);
    let _ = ioctl_in_out(file_handle.handle(), IOCTL_SCSI_PASS_THROUGH_DIRECT, command_buffer.as_mut_slice())?;
    check_sense_info(command_buffer.command.ScsiStatus, &command_buffer.sense_info)
}

#[repr(C)]
struct CommandWithSense {
    pub command: SCSI_PASS_THROUGH_DIRECT,
    pub sense_info: [u8; Self::SENSE_LENGTH as usize],
}

impl CommandWithSense {
    const SENSE_LENGTH: u8 = DEFAULT_SENSE_LENGTH;
    pub fn new(command: SCSI_PASS_THROUGH_DIRECT) -> Self {
        let command = SCSI_PASS_THROUGH_DIRECT {
            SenseInfoOffset: core::mem::offset_of!(CommandWithSense, sense_info) as u32,
            SenseInfoLength: Self::SENSE_LENGTH,
            ..command
        };
        Self { command, sense_info: [0; Self::SENSE_LENGTH as usize] }
    }

    pub const fn size() -> usize {
        core::mem::size_of::<Self>()
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self as *mut Self as *mut u8, Self::size()) }
    }
}

fn check_sense_info(scsi_result: u8, sense_info: &[u8]) -> Result<(), DeviceError> {
    if scsi_result != 0 {
        let raw_response_code = sense_info[0] & 0b0111_1111; // Bit 7 is reserved. See the sense info data structures in the shared scsi `mod`.
        let response_code = SenseResponseCode::try_from(raw_response_code).unwrap_or(SenseResponseCode::Unrecognized);
        match response_code {
            SenseResponseCode::CurrentFixed => Err(parse_fixed_sense_info(sense_info)),
            SenseResponseCode::DeferredFixed => Ok(()),
            SenseResponseCode::CurrentDescriptor => Err(parse_descriptor_sense_info(sense_info)),
            SenseResponseCode::DeferredDescriptor => Ok(()),
            SenseResponseCode::VendorSpecific => {
                Err(SCSIError { sense_key: SenseKey::VendorSpecific, ..Default::default() })
            }
            _ => Err(SCSIError { parse_failed: true, ..Default::default() }),
        }
        .map_err(|err| DeviceError::SCSIError(err))
    } else {
        Ok(())
    }
}

fn parse_fixed_sense_info(sense_info: &[u8]) -> SCSIError {
    let Ok(sense_data) = FixedSenseData::from_bytes(sense_info.into()) else {
        return SCSIError { parse_failed: true, ..Default::default() };
    };
    SCSIError {
        sense_key: sense_data.sense_key,
        additional_sense_code: sense_data.additional_sense_code,
        additional_sense_code_qualifier: sense_data.additional_sense_code_qualifier,
        ..Default::default()
    }
}

fn parse_descriptor_sense_info(sense_info: &[u8]) -> SCSIError {
    let Ok(sense_data) = DescriptorSenseData::from_bytes(sense_info.into()) else {
        return SCSIError { parse_failed: true, ..Default::default() };
    };
    SCSIError {
        sense_key: sense_data.sense_key,
        additional_sense_code: sense_data.additional_sense_code,
        additional_sense_code_qualifier: sense_data.additional_sense_code_qualifier,
        ..Default::default()
    }
}

const DEFAULT_SENSE_LENGTH: u8 = 128;

/// Align the IOCTL buffers to 8 bytes. I don't fully understand this, because
/// the docs (for WinAPI SCSI_PASS_THROUGH_DIRECT) mention "cache alignment", but
/// is that the CPU cache or some other cache? They also mention using
/// the StorageAdapterProperty IOCTL query to get the alignment, and they state
/// that the alignment is one of 1, 2, 4, or 8.
const ALIGNMENT: usize = 8;

/// Pad the size of the data to be a multiple of 512. This is because the
/// INC_512 flag needs to be on for some security protocols, required
/// a buffer of a multiple of 512 bytes.
const PADDING: usize = 512;

/// Get the required INC_512 flag for SCSI security protocol in/out commands.
///
/// The values can be found in the TCG Storage Interface Interactions Specification.
const fn get_inc_512_flag(security_protocol: u8) -> bool {
    match security_protocol {
        0x00 => true,
        0x01 => true,
        0x02 => true,
        0x06 => false,
        _ => panic!("unknown security protocol"),
    }
}
