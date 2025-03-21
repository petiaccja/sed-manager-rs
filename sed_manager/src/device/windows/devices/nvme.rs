//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::device::device::{Device, Interface};
use crate::device::shared::aligned_array::AlignedArray;
use crate::device::shared::memory::write_nonoverlapping;
use crate::device::shared::nvme::IdentifyController;
use crate::device::windows::utility::file_handle::FileHandle;
use crate::device::windows::utility::ioctl::{ioctl_in_out, STORAGE_PROTOCOL_SPECIFIC_DATA, STORAGE_PROTOCOL_TYPE};
use crate::device::windows::Error as WindowsError;
use crate::device::Error as DeviceError;
use crate::serialization::{Deserialize, InputStream, Seek as _, SeekFrom};

use core::mem::offset_of;
use std::os::windows::raw::HANDLE;
use winapi::shared::winerror::ERROR_INVALID_DATA;
use winapi::um::winioctl::{IOCTL_STORAGE_QUERY_PROPERTY, STORAGE_PROPERTY_QUERY};

use super::scsi;
use super::GenericDevice;

pub struct NVMeDevice {
    file: FileHandle,
    cached_desc: IdentifyController,
}

impl NVMeDevice {
    #[allow(unused)]
    pub fn open(path: &str) -> Result<Self, DeviceError> {
        let file = FileHandle::open(path)?;
        let desc = identify_controller(file.handle())?;
        Ok(Self { file, cached_desc: desc })
    }
}

impl TryFrom<GenericDevice> for NVMeDevice {
    type Error = DeviceError;
    fn try_from(value: GenericDevice) -> Result<Self, Self::Error> {
        if Interface::NVMe == value.interface() {
            let desc = identify_controller(value.get_file().handle())?;
            Ok(Self { file: value.take_file(), cached_desc: desc })
        } else {
            Err(DeviceError::InterfaceNotSupported)
        }
    }
}

impl Device for NVMeDevice {
    fn path(&self) -> Option<String> {
        Some(self.file.path().into())
    }

    fn interface(&self) -> Interface {
        Interface::NVMe
    }

    fn model_number(&self) -> String {
        self.cached_desc.model_number_as_str()
    }

    fn serial_number(&self) -> String {
        self.cached_desc.serial_number_as_str()
    }

    fn firmware_revision(&self) -> String {
        self.cached_desc.firmware_revision_as_str()
    }

    fn is_security_supported(&self) -> bool {
        self.cached_desc.security_send_receive_supported
    }

    fn security_send(&self, security_protocol: u8, protocol_specific: [u8; 2], data: &[u8]) -> Result<(), DeviceError> {
        if !self.is_security_supported() {
            return Err(DeviceError::SecurityNotSupported);
        }
        let aligned_data = AlignedArray::from_slice(data, 8).unwrap();
        let protocol_specific = u16::from_be_bytes(protocol_specific);
        Ok(scsi::security_protocol_out(
            &self.file,
            security_protocol,
            protocol_specific,
            aligned_data.as_padded_slice(),
            SCSI_TRANSLATION_INC_512,
        )?)
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
        let mut data = AlignedArray::zeroed(len, 8).unwrap();
        let protocol_specific = u16::from_be_bytes(protocol_specific);
        scsi::security_protocol_in(
            &self.file,
            security_protocol,
            protocol_specific,
            data.as_padded_mut_slice(),
            SCSI_TRANSLATION_INC_512,
        )?;
        Ok(data.into_vec())
    }
}

fn identify_controller(handle: HANDLE) -> Result<IdentifyController, WindowsError> {
    const NVME_MAX_LOG_SIZE: usize = 0x1000;
    let mut buffer = AlignedArray::zeroed(NVME_MAX_LOG_SIZE + 128, 8).unwrap();
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

    let mut stream = InputStream::from(buffer.into_vec());
    stream.seek(SeekFrom::Start((data_offset + response_offset) as u64)).unwrap();
    IdentifyController::deserialize(&mut stream).map_err(|_| WindowsError::Win32(ERROR_INVALID_DATA))
}

/// The value of the INC_512 flag for SCSI to NVMe translation.
///
/// The value of this flag can be found in the NVM Express: SCSI Translation Reference.
const SCSI_TRANSLATION_INC_512: bool = false;

#[cfg(test)]
mod test {
    use super::*;

    use skip_test::may_skip;

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
        let _nvme_drives = get_nvme_drives();
        Ok(())
    }
}
