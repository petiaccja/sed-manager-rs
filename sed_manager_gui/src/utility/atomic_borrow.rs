use std::cell::RefCell;
use std::ops::{Deref, DerefMut};

pub struct AtomicBorrow<T> {
    value: RefCell<T>,
}

impl<T> AtomicBorrow<T> {
    pub fn new(value: T) -> Self {
        Self { value: value.into() }
    }

    pub fn with<R>(&self, updater_fn: impl FnOnce(&T) -> R) -> R {
        updater_fn(self.value.borrow_mut().deref())
    }

    pub fn with_mut<R>(&self, updater_fn: impl FnOnce(&mut T) -> R) -> R {
        updater_fn(self.value.borrow_mut().deref_mut())
    }
}
