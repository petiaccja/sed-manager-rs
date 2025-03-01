use super::ui;

pub struct NativeDeviceIdentity {
    pub name: String,
    pub serial: String,
    pub path: String,
    pub firmware: String,
    pub interface: String,
}

pub struct NativeLockingRange {
    pub name: String,
    pub start_lba: u64,
    pub end_lba: u64,
    pub read_lock_enabled: bool,
    pub write_lock_enabled: bool,
    pub read_locked: bool,
    pub write_locked: bool,
}

impl From<NativeDeviceIdentity> for ui::DeviceIdentity {
    fn from(value: NativeDeviceIdentity) -> Self {
        Self::new(value.name, value.serial, value.path, value.firmware, value.interface)
    }
}

impl From<NativeLockingRange> for ui::LockingRange {
    fn from(value: NativeLockingRange) -> Self {
        Self::new(
            value.name,
            value.start_lba,
            value.end_lba,
            value.read_lock_enabled,
            value.write_lock_enabled,
            value.read_locked,
            value.write_locked,
        )
    }
}
