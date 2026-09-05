use std::pin::Pin;

use pin_project::pin_project;

#[repr(transparent)]
#[pin_project]
#[allow(unused)]
pub struct SyncWrapper<T> {
    #[pin]
    inner: T,
}

// Safety: no method below hands out `&T`.
unsafe impl<T> Sync for SyncWrapper<T> {}

#[allow(unused)]
impl<T> SyncWrapper<T> {
    pub fn new(value: T) -> Self {
        Self { inner: value }
    }

    pub fn as_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    pub fn into_inner(self) -> T {
        self.inner
    }

    pub fn as_pin_mut(self: Pin<&mut Self>) -> Pin<&mut T> {
        self.project().inner
    }
}
