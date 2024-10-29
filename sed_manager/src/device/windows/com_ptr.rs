use std::{ops::Deref, ptr::null_mut};

use winapi::um::unknwnbase::IUnknown;

pub struct ComPtr<T: Deref<Target = IUnknown>> {
    ptr: *mut T,
}

impl<T: Deref<Target = IUnknown>> ComPtr<T> {
    pub fn null() -> Self {
        Self { ptr: null_mut() }
    }
    pub fn get(&self) -> *mut T {
        self.ptr
    }
    pub fn as_mut(&mut self) -> &mut *mut T {
        &mut self.ptr
    }
}

impl<T> Drop for ComPtr<T>
where
    T: Deref<Target = IUnknown>,
{
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() {
                (*self.ptr).Release();
            }
        }
    }
}
