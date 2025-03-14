use crate::device::Error as DeviceError;

pub fn list_physical_drives() -> Result<Vec<String>, DeviceError> {
    Ok(vec![])
}
