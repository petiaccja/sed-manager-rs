//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::mem::offset_of;
use std::path::Path;

use crate::Error as DeviceError;
use crate::shared::aligned_array::AlignedArray;
use crate::shared::memory::write_nonoverlapping;
use crate::shared::nvme::IdentifyController;
use crate::storage_device::{Interface, StorageDevice};
use crate::windows::devices::generic::{DeviceDesc, GenericIoctlDevice};
use crate::windows::ioctl_device::IoctlDevice;

use sorbit::ser_de::FromBytes as _;
use windows::Win32::System::Ioctl::{
    IOCTL_STORAGE_QUERY_PROPERTY, PropertyStandardQuery, ProtocolTypeNvme, STORAGE_PROPERTY_QUERY,
    STORAGE_PROTOCOL_SPECIFIC_DATA, StorageAdapterProtocolSpecificProperty,
};

use super::GenericDevice;

pub use ioctl::NvmeIoctlDevice;

pub struct NvmeDevice {
    ioctl_device: IoctlDevice,
    generic_desc: DeviceDesc,
    desc: IdentifyController,
}

impl NvmeDevice {
    #[allow(unused)]
    pub async fn open(path: &str) -> Result<Self, DeviceError> {
        let ioctl_device = IoctlDevice::open(path).await?;
        let generic_desc = ioctl_device.description().await?;
        let desc = ioctl_device.identify_controller().await?;
        Ok(Self { ioctl_device, generic_desc, desc })
    }

    pub async fn from_generic(value: GenericDevice) -> Result<Self, DeviceError> {
        if Interface::NVMe == value.interface() {
            let ioctl_device = value.ioctl_device();
            let generic_desc = ioctl_device.description().await?;
            let desc = ioctl_device.identify_controller().await?;
            Ok(Self { ioctl_device: value.into_ioctl_device(), generic_desc, desc })
        } else {
            Err(DeviceError::InterfaceNotSupported)
        }
    }
}

#[async_trait::async_trait]
impl StorageDevice for NvmeDevice {
    fn path(&self) -> Option<&Path> {
        Some(&self.ioctl_device.path())
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
        let protocol_specific = u16::from_be_bytes(protocol_specific);
        let buffer = AlignedArray::from_slice(data, 8).unwrap();
        self.ioctl_device
            .security_send(security_protocol, protocol_specific, buffer.as_padded_slice())
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
        let protocol_specific = u16::from_be_bytes(protocol_specific);
        let mut buffer = AlignedArray::zeroed(len, 8).unwrap();
        self.ioctl_device
            .security_receive(security_protocol, protocol_specific, buffer.as_padded_mut_slice())
            .await?;
        Ok(buffer.into_vec())
    }
}

mod ioctl {
    use super::*;
    use crate::windows::devices::scsi::ScsiIoctlDevice as _;

    pub trait NvmeIoctlDevice {
        async fn identify_controller(&self) -> Result<IdentifyController, DeviceError>;

        async fn security_send(
            &self,
            security_protocol: u8,
            security_protocol_specific: u16,
            data_out: &[u8],
        ) -> Result<(), DeviceError>;

        async fn security_receive(
            &self,
            security_protocol: u8,
            security_protocol_specific: u16,
            data_in: &mut [u8],
        ) -> Result<(), DeviceError>;
    }

    impl NvmeIoctlDevice for IoctlDevice {
        async fn identify_controller(&self) -> Result<IdentifyController, DeviceError> {
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

            let _ = self.ioctl_symmetric(IOCTL_STORAGE_QUERY_PROPERTY, &mut buffer).await?;

            let identify_ctrl_buffer = &buffer[(data_offset + response_offset)..];
            IdentifyController::from_bytes(identify_ctrl_buffer).map_err(|_| DeviceError::InvalidArgument)
        }

        async fn security_send(
            &self,
            security_protocol: u8,
            security_protocol_specific: u16,
            data_out: &[u8],
        ) -> Result<(), DeviceError> {
            self.security_protocol_out(
                security_protocol,
                security_protocol_specific,
                data_out,
                SCSI_TRANSLATION_INC_512,
            )
            .await
        }

        async fn security_receive(
            &self,
            security_protocol: u8,
            security_protocol_specific: u16,
            data_in: &mut [u8],
        ) -> Result<(), DeviceError> {
            self.security_protocol_in(security_protocol, security_protocol_specific, data_in, SCSI_TRANSLATION_INC_512)
                .await
        }
    }

    /// The value of the INC_512 flag for SCSI to NVMe translation.
    /// Specified in the NVM Express: SCSI Translation Reference.
    const SCSI_TRANSLATION_INC_512: bool = false;
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::windows::device_list::list_storage_devices;

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
