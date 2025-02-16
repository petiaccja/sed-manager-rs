use crate::device::device::{Device, Interface};
use crate::device::shared::memory::write_nonoverlapping;
use crate::device::shared::nvme::{IdentifyController, Opcode};
use crate::device::windows::utility::file_handle::FileHandle;
use crate::device::windows::utility::ioctl::{ioctl_in_out, STORAGE_PROTOCOL_SPECIFIC_DATA, STORAGE_PROTOCOL_TYPE};
use crate::device::windows::Error as WindowsError;
use crate::device::Error as DeviceError;
use crate::serialization::{Deserialize, InputStream};

use std::io::Seek;
use std::mem::offset_of;
use std::os::windows::raw::HANDLE;
use std::sync::OnceLock;
use winapi::um::winioctl::{IOCTL_STORAGE_QUERY_PROPERTY, STORAGE_PROPERTY_QUERY};

use super::scsi::{
    ioctl_scsi_passthrough_direct, SCSIPassthroughDirection, DEFAULT_SENSE_LENGTH, DEFAULT_SENSE_OFFSET,
};
use super::GenericDevice;

pub struct NVMeDevice {
    file: FileHandle,
    cached_identity: OnceLock<IdentifyController>,
}

impl NVMeDevice {
    #[allow(unused)]
    pub fn open(path: &str) -> Result<Self, DeviceError> {
        let file = FileHandle::open(path)?;
        Ok(Self { file, cached_identity: OnceLock::new() })
    }

    fn identify_controller(&self) -> Result<&IdentifyController, DeviceError> {
        match self.cached_identity.get() {
            Some(identity) => Ok(identity),
            None => {
                let identity = identify_controller(self.file.handle())?;
                Ok(self.cached_identity.get_or_init(|| identity))
            }
        }
    }
}

impl Device for NVMeDevice {
    fn path(&self) -> Option<String> {
        Some(self.file.path().into())
    }

    fn interface(&self) -> Result<Interface, DeviceError> {
        Ok(Interface::NVMe)
    }

    fn model_number(&self) -> Result<String, DeviceError> {
        Ok(self.identify_controller()?.model_number_as_str().trim().to_string())
    }

    fn serial_number(&self) -> Result<String, DeviceError> {
        Ok(self.identify_controller()?.serial_number_as_str().trim().to_string())
    }

    fn firmware_revision(&self) -> Result<String, DeviceError> {
        Ok(self.identify_controller()?.firmware_revision_as_str().trim().to_string())
    }

    fn security_send(&self, security_protocol: u8, protocol_specific: [u8; 2], data: &[u8]) -> Result<(), DeviceError> {
        // This is kind of annoying, because security send will NOT modify this data.
        // Still have to copy it to make it mutable for the borrow checker.
        let mut copy = Vec::from(data);
        let opcode = Opcode::SecuritySend;
        Ok(security_command(self.file.handle(), &mut copy, opcode, security_protocol, protocol_specific)?)
    }

    fn security_recv(
        &self,
        security_protocol: u8,
        protocol_specific: [u8; 2],
        len: usize,
    ) -> Result<Vec<u8>, DeviceError> {
        let mut data = vec![0_u8; len];
        let opcode = Opcode::SecurityReceive;
        security_command(self.file.handle(), &mut data, opcode, security_protocol, protocol_specific)?;
        Ok(data)
    }
}

impl TryFrom<GenericDevice> for NVMeDevice {
    type Error = GenericDevice;
    fn try_from(value: GenericDevice) -> Result<Self, Self::Error> {
        if let Ok(Interface::NVMe) = value.interface() {
            Ok(Self { file: value.take_file(), cached_identity: OnceLock::new() })
        } else {
            Err(value)
        }
    }
}

fn identify_controller(handle: HANDLE) -> Result<IdentifyController, WindowsError> {
    const NVME_MAX_LOG_SIZE: usize = 0x1000;
    let mut buffer = vec![0u8; NVME_MAX_LOG_SIZE + 128];
    let data_offset = offset_of!(STORAGE_PROPERTY_QUERY, AdditionalParameters);
    let response_offset = size_of::<STORAGE_PROTOCOL_SPECIFIC_DATA>();

    let query = STORAGE_PROPERTY_QUERY {
        PropertyId: 49, // StorageAdapterProtocolSpecificProperty
        QueryType: 0,   // PropertyStandardQuery
        AdditionalParameters: [0],
    };

    let data = STORAGE_PROTOCOL_SPECIFIC_DATA {
        ProtocolType: STORAGE_PROTOCOL_TYPE::ProtocolTypeNvme,
        DataType: 1,                 // NVMeDataTypeIdentify
        ProtocolDataRequestValue: 1, // NVME_IDENTIFY_CNS_CONTROLLER
        ProtocolDataRequestSubValue: 0,
        ProtocolDataOffset: response_offset as u32,
        ProtocolDataLength: NVME_MAX_LOG_SIZE as u32,
        FixedProtocolReturnData: 0,
        ProtocolDataRequestSubValue2: 0,
        ProtocolDataRequestSubValue3: 0,
        ProtocolDataRequestSubValue4: 0,
    };

    write_nonoverlapping(&query, &mut buffer);
    write_nonoverlapping(&data, &mut buffer[data_offset..]);

    let _ = ioctl_in_out(handle, IOCTL_STORAGE_QUERY_PROPERTY, &mut buffer)?;

    let mut stream = InputStream::from(buffer);
    stream.seek(std::io::SeekFrom::Start((data_offset + response_offset) as u64)).unwrap();
    match IdentifyController::deserialize(&mut stream) {
        Ok(identity) => Ok(identity),
        Err(_) => panic!("controller identify structure deserialization should not fail"),
    }
}

fn scsi_passthrough_cdb(
    opcode: Opcode,
    security_protocol: u8,
    protocol_specific: [u8; 2],
    buffer_len: usize,
) -> [u8; 16] {
    assert!(buffer_len <= u32::MAX as usize);

    let cdb0 = match opcode {
        Opcode::SecuritySend => 0xB5,
        Opcode::SecurityReceive => 0xA2,
        _ => panic!("opcode not supported with SCSI passthrough"),
    };

    [
        cdb0,
        security_protocol,
        protocol_specific[0],
        protocol_specific[1],
        0,
        0,
        ((buffer_len >> 24) & 0xFF) as u8,
        ((buffer_len >> 16) & 0xFF) as u8,
        ((buffer_len >> 8) & 0xFF) as u8,
        ((buffer_len) & 0xFF) as u8,
        0,
        0,
        0,
        0,
        0,
        0,
    ]
}

fn security_command(
    device: HANDLE,
    data: &mut [u8],
    opcode: Opcode,
    security_protocol: u8,
    protocol_specific: [u8; 2],
) -> Result<(), WindowsError> {
    let cdb = scsi_passthrough_cdb(opcode, security_protocol, protocol_specific, data.len());
    let direction = match opcode {
        Opcode::SecuritySend => SCSIPassthroughDirection::Out,
        Opcode::SecurityReceive => SCSIPassthroughDirection::In,
        _ => panic!("opcode {:?} should not be used with security commands", opcode),
    };
    ioctl_scsi_passthrough_direct(device, data, direction, cdb, DEFAULT_SENSE_LENGTH, DEFAULT_SENSE_OFFSET)
        .map_err(|e| e.into())
}

#[cfg(test)]
mod test {
    use super::*;

    use skip_test::{may_skip, skip, skip_or_unwrap};

    use crate::device::windows::drive_list::list_physical_drives;

    fn get_nvme_drives() -> Vec<NVMeDevice> {
        let drives_paths = list_physical_drives().ok().unwrap_or(vec![]);
        drives_paths
            .into_iter()
            .filter_map(|path| GenericDevice::open(&path).ok())
            .filter_map(|dev| NVMeDevice::try_from(dev).ok())
            .collect()
    }

    #[test]
    #[may_skip]
    fn test_nvme_identify_controller() -> Result<(), DeviceError> {
        let nvme_drives = get_nvme_drives();
        let device = skip_or_unwrap!(nvme_drives.first());
        match device.identify_controller() {
            Ok(_) => Ok(()),
            Err(DeviceError::PermissionDenied) => skip!(),
            Err(err) => Err(err.into()),
        }
    }
}
