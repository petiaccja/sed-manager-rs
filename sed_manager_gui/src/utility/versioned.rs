use core::ops::Deref;
use std::sync::{Arc, Weak};

pub struct Versioned<T: ?Sized> {
    value: Arc<T>,
}

pub struct Current<T: ?Sized> {
    value: Arc<T>,
}

#[derive(Clone)]
pub struct Snapshot<T: ?Sized> {
    value: Weak<T>,
}

impl<T: ?Sized> Versioned<T> {
    pub fn new(value: T) -> Self
    where
        T: Sized,
    {
        Self { value: Arc::new(value) }
    }

    pub fn snapshot(&self) -> Snapshot<T> {
        Snapshot { value: Arc::downgrade(&self.value) }
    }

    pub fn current(&self) -> Current<T> {
        Current { value: self.value.clone() }
    }

    pub fn arc(&self) -> Arc<T> {
        self.value.clone()
    }
}

impl<T: ?Sized> Snapshot<T> {
    pub fn run_if_current<Task, Output>(&self, current: Current<T>, updater: Task) -> Option<Output>
    where
        Task: FnOnce() -> Output,
    {
        if let Some(strong) = self.value.upgrade() {
            if Arc::ptr_eq(&strong, &current.value) {
                Some(updater())
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl<T: ?Sized> Versioned<T> {}

impl<T: ?Sized> From<Arc<T>> for Versioned<T> {
    fn from(value: Arc<T>) -> Self {
        Self { value }
    }
}

impl<T: ?Sized> From<Box<T>> for Versioned<T> {
    fn from(value: Box<T>) -> Self {
        Self { value: value.into() }
    }
}

impl<T: ?Sized> Deref for Versioned<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.value.deref()
    }
}
