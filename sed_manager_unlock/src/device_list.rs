use std::sync::Arc;

use sed_manager::device::{list_physical_drives, open_device, Device, Error as DeviceError};
use sed_manager::messaging::discovery::{Discovery, LockingDescriptor};
use sed_manager::rpc::discover;

pub struct DeviceList {
    pub shadowed: Vec<(Arc<dyn Device>, Discovery)>,
    pub locked: Vec<(Arc<dyn Device>, Discovery)>,
    pub non_locked: Vec<Box<dyn Device>>,
    pub failed: Vec<(String, DeviceError)>,
}

impl DeviceList {
    pub fn new() -> Self {
        Self { shadowed: vec![], locked: vec![], non_locked: vec![], failed: vec![] }
    }

    pub fn query() -> Result<Self, DeviceError> {
        let mut device_list = DeviceList::new();
        let paths = list_physical_drives()?;
        for path in paths {
            let device = match open_device(&path) {
                Ok(device) => device,
                Err(error) => {
                    device_list.failed.push((path, error));
                    continue;
                }
            };
            let Ok(discovery) = discover(&*device) else {
                device_list.non_locked.push(device);
                continue;
            };
            let Some(locking_desc) = discovery.get::<LockingDescriptor>() else {
                device_list.non_locked.push(device);
                continue;
            };
            if !locking_desc.mbr_done && locking_desc.mbr_enabled {
                device_list.shadowed.push((Arc::from(device), discovery));
            } else if locking_desc.locked {
                device_list.locked.push((Arc::from(device), discovery));
            } else {
                device_list.non_locked.push(device);
            }
        }
        Ok(device_list)
    }
}

impl core::fmt::Display for DeviceList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Devices:")?;
        for (device, _) in &self.shadowed {
            writeln!(f, "[S] {} / {}", device.model_number(), device.serial_number())?;
        }

        for (device, _) in &self.locked {
            writeln!(f, "[L] {} / {}", device.model_number(), device.serial_number())?;
        }

        for device in &self.non_locked {
            writeln!(f, "    {} / {}", device.model_number(), device.serial_number())?;
        }

        for (path, error) in &self.failed {
            writeln!(f, "    {} / {}", path, error)?;
        }
        write!(f, "* S: shadowed, L: locked")?;

        Ok(())
    }
}
