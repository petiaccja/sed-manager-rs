use core::ops::{Deref, DerefMut};
use core::ptr::{null_mut, NonNull};
use std::alloc::{alloc, dealloc, handle_alloc_error, Layout, LayoutError};

pub struct AlignedArray {
    layout: Layout,
    data: *mut u8,
    len: usize,
}

impl AlignedArray {
    #[allow(unused)]
    pub fn new() -> Self {
        Self { layout: Layout::from_size_align(0, 1).unwrap(), data: null_mut(), len: 0 }
    }

    pub fn zeroed(len: usize, align: usize) -> Result<Self, LayoutError> {
        Self::filled(0, len, align)
    }

    pub fn zeroed_padded(len: usize, align: usize, pad: usize) -> Result<Self, LayoutError> {
        Self::filled_padded(0, len, align, pad)
    }

    pub fn filled(value: u8, len: usize, align: usize) -> Result<Self, LayoutError> {
        Self::filled_padded(value, len, align, align)
    }

    pub fn filled_padded(value: u8, len: usize, align: usize, pad: usize) -> Result<Self, LayoutError> {
        let capacity = (len + pad - 1) / pad * pad;
        let layout = Layout::from_size_align(capacity, align)?;
        let mut aligned_array = {
            // This block can leak memory if you are not careful!
            let data = unsafe { alloc(layout) };
            if data.is_null() {
                handle_alloc_error(layout);
            }
            Self { layout, data, len }
        };
        aligned_array.as_mut_slice().fill(value);
        aligned_array.as_padded_mut_slice()[len..].fill(0);
        Ok(aligned_array)
    }

    #[allow(unused)]
    pub fn from_slice(data: &[u8], align: usize) -> Result<Self, LayoutError> {
        Self::from_slice_padded(data, align, align)
    }

    pub fn from_slice_padded(data: &[u8], align: usize, pad: usize) -> Result<Self, LayoutError> {
        let mut aligned_array = Self::filled_padded(0, data.len(), align, pad)?;
        aligned_array.copy_from_slice(data);
        Ok(aligned_array)
    }

    /// View as a slice that excludes the aligning padding beyond the length.
    #[allow(unused)]
    pub fn as_slice(&self) -> &[u8] {
        self.deref()
    }

    /// View as a mutable slice that excludes the aligning padding beyond the length.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.deref_mut()
    }

    /// View as a slice that includes the aligning padding beyond the length.
    pub fn as_padded_slice(&self) -> &[u8] {
        let ptr = if !self.data.is_null() { self.data } else { NonNull::dangling().as_ptr() };
        unsafe { core::slice::from_raw_parts(ptr, self.layout.size()) }
    }

    /// View as a mutable slice that includes the aligning padding beyond the length.
    pub fn as_padded_mut_slice(&mut self) -> &mut [u8] {
        let ptr = if !self.data.is_null() { self.data } else { NonNull::dangling().as_ptr() };
        unsafe { core::slice::from_raw_parts_mut(ptr, self.layout.size()) }
    }

    /// The length of the array, including alignment padding at the end.
    #[allow(unused)]
    pub fn capacity(&self) -> usize {
        self.layout.size()
    }

    /// Convert the aligned array into a vector.
    ///
    /// Unfortunately, this requires a reallocation and a memcpy at the moment because
    /// Vec's data must be allocated by GlobalAlloc with the default alignment.
    pub fn into_vec(self) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&self);
        v
    }
}

impl Drop for AlignedArray {
    fn drop(&mut self) {
        if !self.data.is_null() {
            unsafe { dealloc(self.data, self.layout) };
        }
    }
}

impl Deref for AlignedArray {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        let ptr = if !self.data.is_null() { self.data } else { NonNull::dangling().as_ptr() };
        unsafe { core::slice::from_raw_parts(ptr, self.len) }
    }
}

impl DerefMut for AlignedArray {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let ptr = if !self.data.is_null() { self.data } else { NonNull::dangling().as_ptr() };
        unsafe { core::slice::from_raw_parts_mut(ptr, self.len) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_slice() {
        let arr = AlignedArray::new();
        assert_eq!(arr.len(), 0);
    }

    #[test]
    fn filled() -> Result<(), LayoutError> {
        let mut arr = AlignedArray::filled(0xEC, 1011, 32)?;
        assert_eq!(arr.as_ptr().addr() % 32, 0);
        assert_eq!(arr.len(), 1011);
        assert_eq!(arr.as_slice().len(), 1011);
        assert_eq!(arr.as_mut_slice().len(), 1011);
        assert_eq!(arr.capacity(), 1024);
        assert_eq!(arr.as_padded_slice().len(), 1024);
        assert_eq!(arr.as_padded_mut_slice().len(), 1024);
        assert!(arr.iter().all(|value| *value == 0xEC));
        Ok(())
    }

    #[test]
    fn filled_padded() -> Result<(), LayoutError> {
        let mut arr = AlignedArray::filled_padded(0xEC, 1011, 8, 32)?;
        assert_eq!(arr.as_ptr().addr() % 8, 0);
        assert_eq!(arr.len(), 1011);
        assert_eq!(arr.as_slice().len(), 1011);
        assert_eq!(arr.as_mut_slice().len(), 1011);
        assert_eq!(arr.capacity(), 1024);
        assert_eq!(arr.as_padded_slice().len(), 1024);
        assert_eq!(arr.as_padded_mut_slice().len(), 1024);
        assert!(arr.iter().all(|value| *value == 0xEC));
        Ok(())
    }

    #[test]
    fn from_slice() -> Result<(), LayoutError> {
        let data = [1, 2, 3, 4, 5];
        let arr = AlignedArray::from_slice(&data, 8)?;
        assert_eq!(arr.as_ptr().addr() % 8, 0);
        assert_eq!(arr.capacity(), 8);
        assert_eq!(arr.len(), data.len());
        assert!(core::iter::zip(data.iter(), arr.iter()).all(|(x, y)| x == y));
        Ok(())
    }

    #[test]
    fn from_slice_padded() -> Result<(), LayoutError> {
        let data = [1, 2, 3, 4, 5];
        let arr = AlignedArray::from_slice_padded(&data, 8, 32)?;
        assert_eq!(arr.as_ptr().addr() % 8, 0);
        assert_eq!(arr.capacity(), 32);
        assert_eq!(arr.len(), data.len());
        assert!(core::iter::zip(data.iter(), arr.iter()).all(|(x, y)| x == y));
        Ok(())
    }

    #[test]
    fn into_vec() -> Result<(), LayoutError> {
        let data = [1, 2, 3, 4, 5];
        let arr = AlignedArray::from_slice(&data, 8)?;
        let vec = arr.into_vec();
        assert_eq!(vec.as_slice(), data.as_slice());
        Ok(())
    }
}
