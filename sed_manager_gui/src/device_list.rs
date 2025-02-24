use sed_manager::device::{list_physical_drives, open_device, Device, Error as DeviceError};
use sed_manager::fake_device::FakeDevice;

use crate::utility::{run_in_thread, Versioned};

pub struct DeviceList {
    pub devices: Vec<Versioned<dyn Device>>,
    pub unavailable_devices: Vec<(String, DeviceError)>,
}

impl DeviceList {
    pub fn new(devices: Vec<Box<dyn Device>>, unavailable_devices: Vec<(String, DeviceError)>) -> Self {
        Self { devices: devices.into_iter().map(|d| Versioned::from(d)).collect(), unavailable_devices }
    }

    pub fn empty() -> Self {
        Self::new(vec![], vec![])
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
