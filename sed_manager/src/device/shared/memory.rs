pub fn write_nonoverlapping<T: Sized>(value: &T, dst: &mut [u8]) {
    assert!(size_of::<T>() <= dst.len());
    unsafe {
        core::ptr::copy_nonoverlapping(value as *const T, dst.as_mut_ptr() as *mut T, 1);
    };
}

/// This function is marked unsafe because `src` may contain illegal representation.
#[allow(unused)]
pub unsafe fn read_nonoverlapping<T: Sized>(src: &[u8], value: &mut T) {
    assert!(size_of::<T>() <= src.len());
    core::ptr::copy_nonoverlapping(src.as_ptr() as *const T, value as *mut T, 1);
}
