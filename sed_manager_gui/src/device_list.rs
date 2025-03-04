use std::sync::Arc;

use sed_manager::device::{list_physical_drives, open_device, Device, Error as DeviceError};
use sed_manager::fake_device::FakeDevice;

use crate::native_data::NativeDeviceIdentity;
use crate::utility::run_in_thread;

pub struct DeviceList {
    pub devices: Vec<Arc<dyn Device>>,
    pub unavailable_devices: Vec<(String, DeviceError)>,
}

impl DeviceList {
    pub fn new(devices: Vec<Box<dyn Device>>, unavailable_devices: Vec<(String, DeviceError)>) -> Self {
        Self { devices: devices.into_iter().map(|d| Arc::from(d)).collect(), unavailable_devices }
    }

    pub async fn query() -> Result<DeviceList, DeviceError> {
        run_in_thread(Self::query_blocking).await
    }

    pub fn query_blocking() -> Result<DeviceList, DeviceError> {
        let device_paths = list_physical_drives()?;
        let maybe_devices: Vec<_> = device_paths
            .into_iter()
            .map(move |path| open_device(&path).map_err(|error| (path, error)))
            .collect();
        let (mut devices, mut unavailable_devices) = (Vec::new(), Vec::new());
        for result in maybe_devices {
            match result {
                Ok(device) => devices.push(device),
                Err(error) => unavailable_devices.push(error),
            }
        }
        #[cfg(debug_assertions)]
        devices.push(Box::new(FakeDevice::new()));
        Ok(Self::new(devices, unavailable_devices))
    }
}

pub async fn get_device_identity(device: Arc<dyn Device>) -> NativeDeviceIdentity {
    run_in_thread(move || NativeDeviceIdentity {
        name: device.model_number().unwrap_or("Unknown model".into()),
        serial: device.serial_number().unwrap_or("Unknown serial".into()),
        path: device.path().unwrap_or("Unknown path".into()),
        firmware: device.firmware_revision().unwrap_or("Unknown firmware".into()),
        interface: device.interface().map(|x| x.to_string()).unwrap_or("Unknown interface".into()),
    })
    .await
}
