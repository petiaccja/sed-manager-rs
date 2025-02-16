use std::ffi::c_void;

use winapi::shared::{
    ntddscsi::{IOCTL_SCSI_PASS_THROUGH_DIRECT, SCSI_IOCTL_DATA_IN, SCSI_IOCTL_DATA_OUT, SCSI_PASS_THROUGH_DIRECT},
    ntdef::HANDLE,
};

use crate::device::{
    shared::memory::write_nonoverlapping,
    windows::{utility::ioctl::ioctl_in_out, Error},
};

const PTR_LENGTH: usize = size_of::<usize>();
const COMMAND_LENGTH: usize = size_of::<SCSI_PASS_THROUGH_DIRECT>();
pub const DEFAULT_SENSE_LENGTH: u8 = 32;
pub const DEFAULT_SENSE_OFFSET: u32 = ((COMMAND_LENGTH + PTR_LENGTH - 1) / PTR_LENGTH * PTR_LENGTH) as u32;

pub enum SCSIPassthroughDirection {
    /// IF-RECV
    In,
    /// IF-SEND
    Out,
}

fn extend_cdb<const CDB_LEN: usize>(command_descriptor_block: [u8; CDB_LEN]) -> [u8; 16] {
    let mut extended_cbd = [0; 16];
    extended_cbd[..command_descriptor_block.len()].copy_from_slice(&command_descriptor_block);
    extended_cbd
}

fn convert_direction(direction: SCSIPassthroughDirection) -> u8 {
    match direction {
        SCSIPassthroughDirection::In => SCSI_IOCTL_DATA_IN,
        SCSIPassthroughDirection::Out => SCSI_IOCTL_DATA_OUT,
    }
}

pub fn ioctl_scsi_passthrough_direct<const CDB_LEN: usize>(
    device: HANDLE,
    data: &mut [u8],
    direction: SCSIPassthroughDirection,
    command_descriptor_block: [u8; CDB_LEN],
    sense_length: u8,
    sense_offset: u32,
) -> Result<(), Error> {
    assert!(sense_offset >= DEFAULT_SENSE_OFFSET);
    let mut command_buffer = Vec::new();
    command_buffer.resize(sense_offset as usize + sense_length as usize, 0);

    let command = SCSI_PASS_THROUGH_DIRECT {
        Length: size_of::<SCSI_PASS_THROUGH_DIRECT>() as u16,
        ScsiStatus: 0,
        PathId: 0,
        TargetId: 1,
        Lun: 0,
        CdbLength: 12,
        SenseInfoLength: sense_length,
        DataIn: convert_direction(direction),
        DataTransferLength: data.len() as u32,
        TimeOutValue: 2,
        DataBuffer: data.as_mut_ptr() as *mut c_void,
        SenseInfoOffset: sense_offset,
        Cdb: extend_cdb(command_descriptor_block),
    };

    write_nonoverlapping(&command, &mut command_buffer);
    match ioctl_in_out(device, IOCTL_SCSI_PASS_THROUGH_DIRECT, &mut command_buffer) {
        Ok(_) => Ok(()),
        Err(err) => Err(err.into()),
    }
}
