//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::fs;
use std::path::PathBuf;

use crate::Error as DeviceError;
use crate::linux::Error as LinuxError;

pub fn get_nvme_controller(device: PathBuf) -> PathBuf {
    const PREFIX: &str = "nvme";
    if let Some(name) = device.file_name() {
        let name = name.to_string_lossy().to_string();
        if name.starts_with(PREFIX) {
            let cut = name[PREFIX.len()..].find('n').unwrap_or(name.len()) + PREFIX.len();
            let controller = device.with_file_name(&name[0..cut]);
            if controller.exists() { controller } else { device }
        } else {
            device
        }
    } else {
        device
    }
}

fn list_physical_drives_sync() -> Result<Vec<PathBuf>, DeviceError> {
    const DISK_FOLDER: &str = "/dev/disk/by-id";

    // Get all drives in the by-id folder.
    let drive_iter = fs::read_dir(DISK_FOLDER).map_err(|_| LinuxError::NoDiskFolder)?;
    let drives = drive_iter.filter_map(|entry| entry.ok().map(|entry| entry.path()));

    // Canonicalize all drives: this removes symlinks so we get `/dev/nvme0n1` instead of `/dev/disk/by-id/nvme-****-1`.
    let drives = drives.filter_map(|path| fs::canonicalize(path).ok());

    // We need the NVMe controller, e.g. `/dev/nvme0`, not the namespaces, like `/dev/nvme0n2`.
    let mut drives: Vec<_> = drives.map(get_nvme_controller).collect();

    // Sort and dedup as `by-id` contains duplicates. This removes all entries
    // that are a continuation of another entry, e.g. `/dev/sda1` is removed if
    // `/dev/sda` is present.
    drives.sort();
    drives.dedup_by(|a, b| {
        let a = a.to_string_lossy();
        let b = b.to_string_lossy();
        a.starts_with(b.as_ref()) || b.starts_with(a.as_ref())
    });

    Ok(drives)
}

pub async fn list_physical_drives() -> Result<Vec<PathBuf>, DeviceError> {
    blocking::unblock(list_physical_drives_sync).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_physical_drives() -> Result<(), DeviceError> {
        let drives = match list_physical_drives().await {
            Ok(drives) => drives,
            Err(DeviceError::PlatformError(LinuxError::NoDiskFolder)) => return Ok(()),
            Err(err) => return Err(err),
        };
        // Make sure the NVMe controllers are returned.
        assert!(!drives.iter().any(|dev| dev.to_string_lossy().contains("nvme0n")));
        assert!(!drives.iter().any(|dev| dev.to_string_lossy().contains("nvme1n")));
        assert!(!drives.iter().any(|dev| dev.to_string_lossy().contains("nvme2n")));
        assert!(!drives.iter().any(|dev| dev.to_string_lossy().contains("nvme3n")));
        // Make sure no partitions are returned.
        assert!(!drives.iter().any(|dev| dev.to_string_lossy().contains("sda0")));
        assert!(!drives.iter().any(|dev| dev.to_string_lossy().contains("sda1")));
        assert!(!drives.iter().any(|dev| dev.to_string_lossy().contains("sdb0")));
        assert!(!drives.iter().any(|dev| dev.to_string_lossy().contains("sdb1")));
        Ok(())
    }
}
