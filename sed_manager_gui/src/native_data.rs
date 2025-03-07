use super::ui;

pub struct NativeDeviceIdentity {
    pub name: String,
    pub serial: String,
    pub path: String,
    pub firmware: String,
    pub interface: String,
}

impl From<NativeDeviceIdentity> for ui::DeviceIdentity {
    fn from(value: NativeDeviceIdentity) -> Self {
        Self::new(value.name, value.serial, value.path, value.firmware, value.interface)
    }
}
