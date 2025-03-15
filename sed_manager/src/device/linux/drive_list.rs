use std::fs;
use std::path::PathBuf;

use crate::device::linux::Error as LinuxError;
use crate::device::Error as DeviceError;

pub fn get_nvme_controller(device: PathBuf) -> PathBuf {
    const PREFIX: &str = "nvme";
    if let Some(name) = device.file_name() {
        let name = name.to_string_lossy().to_string();
        if name.starts_with(PREFIX) {
            let cut = name[PREFIX.len()..].find('n').unwrap_or(name.len()) + PREFIX.len();
            let controller = device.with_file_name(&name[0..cut]);
            if controller.exists() {
                controller
            } else {
                device
            }
        } else {
            device
        }
    } else {
        device
    }
}

pub fn list_physical_drives() -> Result<Vec<String>, DeviceError> {
    const DISK_FOLDER: &str = "/dev/disk/by-id";

    // Get all drives in the by-id folder.
    let drive_iter = fs::read_dir(DISK_FOLDER).map_err(|_| LinuxError::NoDiskFolder)?;
    let drives = drive_iter.filter_map(|entry| entry.ok().map(|entry| entry.path()));

    // Canonicalize all drives: this removes symlinks so we get `/dev/nvme0n1` instead of `/dev/disk/by-id/nvme-****-1`.
    let drives = drives.into_iter().filter_map(|path| fs::canonicalize(path).ok());

    // We need the NVMe controller, e.g. `/dev/nvme0`, not the namespaces, like `/dev/nvme0n2`.
    let drives = drives.into_iter().map(|path| get_nvme_controller(path));

    // Convert drives into a String.
    let mut drives: Vec<_> = drives.into_iter().map(|path| path.to_string_lossy().to_string()).collect();

    // Sort and dedup as `by-id` contains duplicates. This removes all entries
    // that are a continuation of another entry, e.g. `/dev/sda1` is removed if
    // `/dev/sda` is present.
    drives.sort();
    drives.dedup_by(|a, b| a.starts_with(b.as_str()) || b.starts_with(a.as_str()));

    Ok(drives)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_physical_drives() -> Result<(), DeviceError> {
        let drives = match list_physical_drives() {
            Ok(drives) => drives,
            Err(DeviceError::PlatformError(LinuxError::NoDiskFolder)) => return Ok(()),
            Err(err) => return Err(err),
        };
        // Make sure the NVMe controllers are returned.
        assert!(!drives.iter().any(|dev| dev.contains("nvme0n")));
        assert!(!drives.iter().any(|dev| dev.contains("nvme1n")));
        assert!(!drives.iter().any(|dev| dev.contains("nvme2n")));
        assert!(!drives.iter().any(|dev| dev.contains("nvme3n")));
        // Make sure no partitions are returned.
        assert!(!drives.iter().any(|dev| dev.contains("sda0")));
        assert!(!drives.iter().any(|dev| dev.contains("sda1")));
        assert!(!drives.iter().any(|dev| dev.contains("sdb0")));
        assert!(!drives.iter().any(|dev| dev.contains("sdb1")));
        Ok(())
    }
}
