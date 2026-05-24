//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::mem::offset_of;
use std::path::Path;

use crate::Error as DeviceError;
use crate::device::{Device, Interface};
use crate::shared::aligned_array::AlignedArray;
use crate::shared::memory::write_nonoverlapping;
use crate::shared::nvme::IdentifyController;
use crate::windows::devices::generic::{GenericDeviceDesc, query_description};
use crate::windows::devices::raw_device::RawDevice;

use sorbit::ser_de::FromBytes as _;
use windows::Win32::System::Ioctl::{
    IOCTL_STORAGE_QUERY_PROPERTY, PropertyStandardQuery, ProtocolTypeNvme, STORAGE_PROPERTY_QUERY,
    STORAGE_PROTOCOL_SPECIFIC_DATA, StorageAdapterProtocolSpecificProperty,
};

use super::GenericDevice;
use super::scsi;

pub struct NvmeDevice {
    raw_device: RawDevice,
    generic_desc: GenericDeviceDesc,
    desc: IdentifyController,
}

impl NvmeDevice {
    #[allow(unused)]
    pub async fn open(path: &str) -> Result<Self, DeviceError> {
        let raw_device = RawDevice::open(path).await?;
        let generic_desc = query_description(&raw_device).await?;
        let desc = identify_controller(&raw_device).await?;
        Ok(Self { raw_device, generic_desc, desc })
    }

    pub async fn from_generic(value: GenericDevice) -> Result<Self, DeviceError> {
        if Interface::NVMe == value.interface() {
            let raw_device = value.into_raw_device();
            let generic_desc = query_description(&raw_device).await?;
            let desc = identify_controller(&raw_device).await?;
            Ok(Self { raw_device, generic_desc, desc })
        } else {
            Err(DeviceError::InterfaceNotSupported)
        }
    }
}

#[async_trait::async_trait]
impl Device for NvmeDevice {
    fn path(&self) -> Option<&Path> {
        Some(&self.raw_device.path())
    }

    fn interface(&self) -> Interface {
        Interface::NVMe
    }

    fn model_number(&self) -> String {
        self.desc.model_number_as_str()
    }

    fn serial_number(&self) -> String {
        self.desc.serial_number_as_str()
    }

    fn firmware_revision(&self) -> String {
        self.desc.firmware_revision_as_str()
    }

    fn is_security_supported(&self) -> bool {
        self.desc.security_send_receive_supported
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
        let aligned_data = AlignedArray::from_slice(data, 8).unwrap();
        let protocol_specific = u16::from_be_bytes(protocol_specific);
        scsi::security_protocol_out(
            &self.raw_device,
            security_protocol,
            protocol_specific,
            aligned_data.as_padded_slice(),
            SCSI_TRANSLATION_INC_512,
        )
        .await
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
        let mut data = AlignedArray::zeroed(len, 8).unwrap();
        let protocol_specific = u16::from_be_bytes(protocol_specific);
        scsi::security_protocol_in(
            &self.raw_device,
            security_protocol,
            protocol_specific,
            data.as_padded_mut_slice(),
            SCSI_TRANSLATION_INC_512,
        )
        .await?;
        Ok(data.into_vec())
    }
}

async fn identify_controller(raw_device: &RawDevice) -> Result<IdentifyController, DeviceError> {
    const NVME_MAX_LOG_SIZE: usize = 0x1000;
    let mut buffer = AlignedArray::zeroed(NVME_MAX_LOG_SIZE + 128, 8).unwrap();
    let data_offset = offset_of!(STORAGE_PROPERTY_QUERY, AdditionalParameters);
    let response_offset = size_of::<STORAGE_PROTOCOL_SPECIFIC_DATA>();

    let query = STORAGE_PROPERTY_QUERY {
        PropertyId: StorageAdapterProtocolSpecificProperty,
        QueryType: PropertyStandardQuery,
        AdditionalParameters: [0],
    };

    let data = STORAGE_PROTOCOL_SPECIFIC_DATA {
        ProtocolType: ProtocolTypeNvme,
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

    let _ = raw_device.device_io_control_symmetric(IOCTL_STORAGE_QUERY_PROPERTY, &mut buffer).await?;

    let identify_ctrl_buffer = &buffer[(data_offset + response_offset)..];
    IdentifyController::from_bytes(identify_ctrl_buffer).map_err(|_| DeviceError::InvalidArgument)
}

/// The value of the INC_512 flag for SCSI to NVMe translation.
///
/// The value of this flag can be found in the NVM Express: SCSI Translation Reference.
const SCSI_TRANSLATION_INC_512: bool = false;

#[cfg(test)]
mod test {
    use super::*;

    use crate::windows::drive_list::list_physical_drives;

    async fn get_nvme_devices() -> Vec<NvmeDevice> {
        let paths = list_physical_drives().await.ok().unwrap_or(vec![]);
        let mut nvme_devices = Vec::new();
        for path in paths {
            if let Ok(generic_device) = GenericDevice::open(&path).await
                && let Ok(nvme_device) = NvmeDevice::from_generic(generic_device).await
            {
                nvme_devices.push(nvme_device);
            }
        }
        nvme_devices
    }

    #[tokio::test]
    async fn test_nvme_identify_controller() -> Result<(), DeviceError> {
        let _nvme_drives = get_nvme_devices().await;
        Ok(())
    }
}
