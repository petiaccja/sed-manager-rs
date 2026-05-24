use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};

pub struct DeviceHandle(pub HANDLE);

impl Drop for DeviceHandle {
    fn drop(&mut self) {
        if self.0 != INVALID_HANDLE_VALUE {
            unsafe {
                let _ = CloseHandle(self.0);
            };
        }
    }
}

unsafe impl Send for DeviceHandle {}
unsafe impl Sync for DeviceHandle {}
