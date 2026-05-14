//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::ops::{Deref, DerefMut};
use std::cell::RefCell;

pub struct PeekCell<T> {
    value: RefCell<T>,
}

impl<T> PeekCell<T> {
    pub fn new(value: T) -> Self {
        Self { value: value.into() }
    }

    pub fn peek<R>(&self, updater_fn: impl FnOnce(&T) -> R) -> R {
        updater_fn(self.value.borrow_mut().deref())
    }

    pub fn peek_mut<R>(&self, updater_fn: impl FnOnce(&mut T) -> R) -> R {
        updater_fn(self.value.borrow_mut().deref_mut())
    }
}
