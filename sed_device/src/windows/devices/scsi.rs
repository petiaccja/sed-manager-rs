//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::ffi::c_void;
use std::mem::transmute;
use std::path::Path;

use sorbit::ser_de::{FromBytes as _, ToBytes as _};
use windows::Win32::Storage::IscsiDisc::{
    IOCTL_SCSI_PASS_THROUGH_DIRECT, SCSI_IOCTL_DATA_IN, SCSI_IOCTL_DATA_OUT, SCSI_PASS_THROUGH_DIRECT,
};

use crate::shared::aligned_array::AlignedArray;
use crate::shared::scsi::{Command, ScsiError, SecurityProtocolIn, SecurityProtocolOut, SenseData, SenseKey};
use crate::windows::devices::raw_device::RawDevice;
use crate::{Device, Error as DeviceError, Interface};

use super::GenericDevice;

pub struct ScsiDevice {
    generic_device: GenericDevice,
}

impl ScsiDevice {
    #[allow(unused)]
    pub async fn open(path: &str) -> Result<Self, DeviceError> {
        // This does not check the interface, you can force SCSI on an unknown device.
        let generic_device = GenericDevice::open(path).await?;
        Ok(Self { generic_device })
    }
}

impl TryFrom<GenericDevice> for ScsiDevice {
    type Error = DeviceError;
    fn try_from(value: GenericDevice) -> Result<Self, Self::Error> {
        if let Interface::SCSI = value.interface() {
            Ok(Self { generic_device: value })
        } else {
            Err(DeviceError::InterfaceNotSupported)
        }
    }
}

#[async_trait::async_trait]
impl Device for ScsiDevice {
    fn path(&self) -> Option<&Path> {
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

    fn is_removable(&self) -> bool {
        self.generic_device.is_removable()
    }

    async fn security_send(
        &self,
        security_protocol: u8,
        protocol_specific: [u8; 2],
        data: &[u8],
    ) -> Result<(), DeviceError> {
        let aligned_data = AlignedArray::from_slice_padded(data, ALIGNMENT, PADDING).unwrap();
        let protocol_specific = u16::from_be_bytes(protocol_specific);
        Ok(security_protocol_out(
            self.generic_device.get_file(),
            security_protocol,
            protocol_specific,
            aligned_data.as_padded_slice(),
            get_inc_512_flag(security_protocol),
        )
        .await?)
    }

    async fn security_recv(
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
        )
        .await?;
        Ok(data.into_vec())
    }
}

pub async fn security_protocol_in(
    file_handle: &RawDevice,
    security_protocol: u8,
    security_protocol_specific: u16,
    data_in: &mut [u8],
    inc_512: bool,
) -> Result<(), DeviceError> {
    let command = Command::from(SecurityProtocolIn::new(
        security_protocol,
        security_protocol_specific,
        data_in.len() as u32,
        inc_512,
    ));
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
        DataIn: SCSI_IOCTL_DATA_IN as u8,
        DataTransferLength: data_in.len() as u32,
        TimeOutValue: 2,
        DataBuffer: data_in.as_mut_ptr() as *mut c_void,
        SenseInfoOffset: 0,
        Cdb: core::array::from_fn(|_| extended_cdb.next().unwrap()),
    };

    let mut request_buffer = make_request_buffer(command);
    let _ = file_handle
        .device_io_control_symmetric(IOCTL_SCSI_PASS_THROUGH_DIRECT, request_buffer.as_mut_slice())
        .await?;
    parse_request_buffer(&request_buffer).map_err(|err| err.into())
}

pub async fn security_protocol_out(
    file_handle: &RawDevice,
    security_protocol: u8,
    security_protocol_specific: u16,
    data_out: &[u8],
    inc_512: bool,
) -> Result<(), DeviceError> {
    let command = Command::from(SecurityProtocolOut::new(
        security_protocol,
        security_protocol_specific,
        data_out.len() as u32,
        inc_512,
    ));
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
        DataIn: SCSI_IOCTL_DATA_OUT as u8,
        DataTransferLength: data_out.len() as u32,
        TimeOutValue: 2,
        DataBuffer: data_out.as_ptr() as *mut c_void, // Data is not actually modified, hence the unsafe cast.
        SenseInfoOffset: 0,
        Cdb: core::array::from_fn(|_| extended_cdb.next().unwrap()),
    };

    let mut request_buffer = make_request_buffer(command);
    let _ = file_handle
        .device_io_control_symmetric(IOCTL_SCSI_PASS_THROUGH_DIRECT, request_buffer.as_mut_slice())
        .await?;
    parse_request_buffer(&request_buffer).map_err(|err| err.into())
}

const REQUEST_BUFFER_LEN: usize = size_of::<SCSI_PASS_THROUGH_DIRECT>() + SenseData::MAX_LEN;

fn make_request_buffer(command: SCSI_PASS_THROUGH_DIRECT) -> [u8; REQUEST_BUFFER_LEN] {
    let command = SCSI_PASS_THROUGH_DIRECT {
        SenseInfoOffset: size_of::<SCSI_PASS_THROUGH_DIRECT>() as u32,
        SenseInfoLength: SenseData::MAX_LEN as u8,
        ..command
    };
    let mut buffer = [0u8; REQUEST_BUFFER_LEN];
    let command_slice: [u8; size_of::<SCSI_PASS_THROUGH_DIRECT>()] = unsafe { transmute(command) };
    buffer[0..command_slice.len()].copy_from_slice(&command_slice);
    buffer
}

fn parse_request_buffer(buffer: &[u8; REQUEST_BUFFER_LEN]) -> Result<(), ScsiError> {
    let command_buffer = &buffer[..size_of::<SCSI_PASS_THROUGH_DIRECT>()];
    let command = unsafe { *(command_buffer.as_ptr() as *const SCSI_PASS_THROUGH_DIRECT) };
    let sense_buffer = &buffer[size_of::<SCSI_PASS_THROUGH_DIRECT>()..];
    if command.ScsiStatus != 0 {
        match SenseData::from_bytes(sense_buffer) {
            Ok(sense_data) => match ScsiError::from(sense_data) {
                ScsiError::Sense { sense_key: SenseKey::NoSense, .. } => Ok(()),
                err => Err(err),
            },
            Err(_) => Err(ScsiError::Parse),
        }
    } else {
        Ok(())
    }
}

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
