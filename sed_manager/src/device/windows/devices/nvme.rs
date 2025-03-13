use crate::device::device::{Device, Interface};
use crate::device::shared::memory::write_nonoverlapping;
use crate::device::shared::nvme::IdentifyController;
use crate::device::windows::utility::file_handle::FileHandle;
use crate::device::windows::utility::ioctl::{ioctl_in_out, STORAGE_PROTOCOL_SPECIFIC_DATA, STORAGE_PROTOCOL_TYPE};
use crate::device::windows::Error as WindowsError;
use crate::device::Error as DeviceError;
use crate::serialization::{Deserialize, InputStream, Seek as _, SeekFrom};

use core::mem::offset_of;
use std::os::windows::raw::HANDLE;
use std::sync::OnceLock;
use winapi::shared::winerror::ERROR_INVALID_DATA;
use winapi::um::winioctl::{IOCTL_STORAGE_QUERY_PROPERTY, STORAGE_PROPERTY_QUERY};

use super::scsi;
use super::GenericDevice;

pub struct NVMeDevice {
    file: FileHandle,
    cached_identity: OnceLock<IdentifyController>,
}

impl NVMeDevice {
    #[allow(unused)]
    pub fn open(path: &str) -> Result<Self, DeviceError> {
        // This does not check the interface, you can force NVMe on an unknown device.
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
        let protocol_specific = u16::from_be_bytes(protocol_specific);
        Ok(scsi::security_protocol_out(
            &self.file,
            security_protocol,
            protocol_specific,
            data,
            SCSI_NVME_TRANSLATION_INC_512,
        )?)
    }

    fn security_recv(
        &self,
        security_protocol: u8,
        protocol_specific: [u8; 2],
        len: usize,
    ) -> Result<Vec<u8>, DeviceError> {
        let mut data = vec![0; len];
        let protocol_specific = u16::from_be_bytes(protocol_specific);
        scsi::security_protocol_in(
            &self.file,
            security_protocol,
            protocol_specific,
            data.as_mut_slice(),
            SCSI_NVME_TRANSLATION_INC_512,
        )?;
        Ok(data)
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
    stream.seek(SeekFrom::Start((data_offset + response_offset) as u64)).unwrap();
    IdentifyController::deserialize(&mut stream).map_err(|_| WindowsError::Win32(ERROR_INVALID_DATA))
}

/// The value of the INC_512 flag for SCSI to NVMe translation.
///
/// The value of this flag can be found in the NVM Express: SCSI Translation Reference.
const SCSI_NVME_TRANSLATION_INC_512: bool = false;

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
