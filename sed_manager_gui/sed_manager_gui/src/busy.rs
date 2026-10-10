//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::device_list::DeviceList;
use crate::ui_ext::{CommandStatusExt, DeviceExt};

/// A subset of the device (e.g. TPer, session). This is used to track which
/// parts of the device are busy and currently in use by the UI commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Resource {
    Tper,
    Session,
}

/// Counts the commands that are currently busy on each resource of each device.
///
/// Multiple commands can be busy on the same resource at the same time (e.g.
/// the TPer, which is shared), so the UI's `busy` flag must only be cleared
/// when the use count drops to zero, meaning no command is using that resource.
#[derive(Debug, Default)]
pub struct BusyCounters {
    counts: RefCell<HashMap<(PathBuf, Resource), usize>>,
}

/// Marks a resource of a device as busy in the UI for the lifetime of the guard.
///
/// The guard only drives the busy indicators of the UI, it does NOT provide
/// mutual exclusion. That is the job of the device entry's and the session's
/// locks. To keep the UI indicators accurate, synchronize the lifetime of this
/// guard and the actual lock guards. For correctness, acquire the lock first,
/// then the `BusyGuard`, and drop the `BusyGuard` first, then the lock.
pub struct BusyGuard<'a> {
    device_list: &'a DeviceList,
    device_id: &'a Path,
    resource: Resource,
}

impl<'a> BusyGuard<'a> {
    pub fn new(device_list: &'a DeviceList, device_id: &'a Path, resource: Resource) -> Self {
        acquire(device_list, device_id, resource);
        Self { device_list, device_id, resource }
    }
}

impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        release(self.device_list, self.device_id, self.resource);
    }
}

fn acquire(device_list: &DeviceList, device_id: &Path, resource: Resource) {
    *device_list.busy.counts.borrow_mut().entry((device_id.to_path_buf(), resource)).or_default() += 1;
    display_resource_status(device_list, device_id, resource, true);
}

fn release(device_list: &DeviceList, device_id: &Path, resource: Resource) {
    let busy = {
        let mut counts = device_list.busy.counts.borrow_mut();
        let key = (device_id.to_path_buf(), resource);
        // Entries are removed when their count drops to zero, so an existing
        // entry always has a count of at least one.
        let Some(count) = counts.get_mut(&key) else {
            debug_assert!(false, "busy counter released without being acquired");
            return;
        };
        *count = count.saturating_sub(1);
        let busy = *count > 0;
        if !busy {
            counts.remove(&key);
        }
        busy
    };

    display_resource_status(device_list, device_id, resource, busy);
}

fn display_resource_status(device_list: &DeviceList, device_id: &Path, resource: Resource, busy: bool) {
    let _ = device_list.ui.update(device_id, |value| {
        let command_status = value.command_status.clone();
        let command_status = match resource {
            Resource::Tper => command_status.with_tper_busy(busy),
            Resource::Session => command_status.with_session_busy(busy),
        };
        value.with_command_status(command_status)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    use sed_manager_gui_slint as ui;

    fn device_list(device_id: &Path) -> DeviceList {
        let mut device_list = DeviceList::default();
        device_list.ui.insert(device_id.to_path_buf(), ui::Device::default());
        device_list
    }

    fn status(device_list: &DeviceList, device_id: &Path) -> (bool, bool) {
        let command_status = device_list.ui.get(device_id).unwrap().command_status;
        (command_status.tper_busy, command_status.session_busy)
    }

    #[test]
    fn single_guard() {
        let device_id = Path::new("/dev/test");
        let device_list = device_list(device_id);

        let guard = BusyGuard::new(&device_list, device_id, Resource::Tper);
        assert_eq!(status(&device_list, device_id), (true, false));
        drop(guard);
        assert_eq!(status(&device_list, device_id), (false, false));
    }

    #[test]
    fn overlapping_guards() {
        let device_id = Path::new("/dev/test");
        let device_list = device_list(device_id);

        let first = BusyGuard::new(&device_list, device_id, Resource::Tper);
        let second = BusyGuard::new(&device_list, device_id, Resource::Tper);
        drop(first);
        assert_eq!(status(&device_list, device_id), (true, false));
        drop(second);
        assert_eq!(status(&device_list, device_id), (false, false));
    }

    #[test]
    fn independent_resources() {
        let device_id = Path::new("/dev/test");
        let device_list = device_list(device_id);

        let tper = BusyGuard::new(&device_list, device_id, Resource::Tper);
        let session = BusyGuard::new(&device_list, device_id, Resource::Session);
        assert_eq!(status(&device_list, device_id), (true, true));
        drop(tper);
        assert_eq!(status(&device_list, device_id), (false, true));
        drop(session);
        assert_eq!(status(&device_list, device_id), (false, false));
    }

    #[test]
    fn missing_device() {
        let device_list = DeviceList::default();
        let device_id = Path::new("/dev/missing");

        let guard = BusyGuard::new(&device_list, device_id, Resource::Tper);
        drop(guard);
        assert!(device_list.busy.counts.borrow().is_empty());
    }
}
