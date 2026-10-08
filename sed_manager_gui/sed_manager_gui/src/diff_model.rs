//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::{
    cell::RefCell,
    sync::atomic::{AtomicBool, Ordering},
};

use slint::{Model, ModelNotify, ModelTracker as _};

/// A model that lets you alter its content while remembering the initial state.
/// You can restore the model to its initial state at any time via [`discard`],
/// or you can commit the current state so that it becomes the new initial state
/// via [`commit`].
///
/// This model is useful for situations where the GUI updates the model, and it
/// has to be written back to the device, but only when it actually changed. It
/// can also be used to prompt the user if they have uncommitted changes they are
/// about to discard.
///
/// [`discard`]: Self::discard
/// [`commit`]: Self::commit
pub struct DiffModel<T> {
    committed: RefCell<Vec<T>>,
    current: RefCell<Vec<T>>,
    notify: ModelNotify,
    is_dirty: AtomicBool,
}

impl<T> DiffModel<T> {
    pub fn new(data: Vec<T>) -> Self
    where
        T: Clone,
    {
        Self {
            committed: data.clone().into(),
            current: data.into(),
            notify: ModelNotify::default(),
            is_dirty: false.into(),
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.notify.track_row_count_changes();
        (0..self.current.borrow().len()).for_each(|row| self.notify.track_row_data_changes(row));
        self.is_dirty.load(Ordering::Relaxed)
    }

    pub fn discard(&self)
    where
        T: Clone,
    {
        *self.current.borrow_mut() = self.committed.borrow().clone();
        self.is_dirty.store(false, Ordering::Relaxed);
        self.notify.reset();
    }

    #[allow(unused)]
    pub fn commit(&self)
    where
        T: Clone,
    {
        *self.committed.borrow_mut() = self.current.borrow().clone();
        self.is_dirty.store(false, Ordering::Relaxed);
    }
}

impl<T> From<Vec<T>> for DiffModel<T>
where
    T: Clone,
{
    fn from(value: Vec<T>) -> Self {
        Self::new(value)
    }
}

impl<T> Model for DiffModel<T>
where
    T: Clone + PartialEq + 'static,
{
    type Data = T;

    fn row_count(&self) -> usize {
        self.current.borrow().len()
    }

    fn row_data(&self, row: usize) -> Option<Self::Data> {
        self.current.borrow().get(row).cloned()
    }

    fn set_row_data(&self, row: usize, data: Self::Data) {
        let mut current = self.current.borrow_mut();
        if let Some(value) = current.get_mut(row)
            && value != &data
        {
            *value = data;
            self.is_dirty.store(*current != *self.committed.borrow(), Ordering::Relaxed);
            // Must drop all mutable borrows before notify is called, or else
            // it will panic as notify also seems to borrow.
            drop(current);
            self.notify.row_changed(row);
        }
    }

    fn model_tracker(&self) -> &dyn slint::ModelTracker {
        &self.notify
    }

    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initially_not_dirty() {
        let value = DiffModel::from(vec![1, 2, 3]);
        assert!(!value.is_dirty());
    }

    #[test]
    fn row_count() {
        let value = DiffModel::from(vec![1, 2, 3]);
        assert_eq!(value.row_count(), 3);
    }

    #[test]
    fn set_row_data_makes_dirty() {
        let value = DiffModel::from(vec![1, 2, 3]);
        value.set_row_data(1, 5);
        assert!(value.is_dirty());
    }

    #[test]
    fn set_row_data_changes_value() {
        let value = DiffModel::from(vec![1, 2, 3]);
        value.set_row_data(1, 5);
        assert_eq!(value.row_data(1), Some(5));
    }

    #[test]
    fn discard_restores_data() {
        let value = DiffModel::from(vec![1, 2, 3]);
        value.set_row_data(1, 5);
        value.discard();
        assert_eq!(value.row_data(1), Some(2));
    }

    #[test]
    fn commit_makes_not_dirty() {
        let value = DiffModel::from(vec![1, 2, 3]);
        value.set_row_data(1, 5);
        value.commit();
        assert_eq!(value.row_data(1), Some(5));
        assert!(!value.is_dirty());
    }
}
