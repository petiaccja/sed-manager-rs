use sed_manager_gui_slint as ui;
use slint::{Model, ModelRc};

use crate::diff_model::DiffModel;

/// Returns whether the contents of the locking ranges have changed compared
/// to the initial contents.
///
/// # Errors
///
/// If the model is not tracking changes, it's always considered dirty and
/// true is returned.
pub fn is_any_dirty(locking_ranges: ModelRc<ui::LockingRange>) -> bool {
    if let Some(diff_model) = locking_ranges.as_any().downcast_ref::<DiffModel<ui::LockingRange>>() {
        diff_model.is_any_dirty()
    } else {
        true
    }
}

/// Returns whether the contents of the individual locking range have changed
/// compared to the initial contents.
///
/// # Errors
///
/// If the model is not tracking changes, it's always considered dirty and
/// true is returned.
pub fn is_individual_dirty(index: i32, locking_ranges: ModelRc<ui::LockingRange>) -> bool {
    if let Some(diff_model) = locking_ranges.as_any().downcast_ref::<DiffModel<ui::LockingRange>>() {
        diff_model.is_individual_dirty(index as usize)
    } else {
        true
    }
}

/// Discards the changes in the contents of the locking ranges.
///
/// # Errors
///
/// If the model is not tracking changes, it returns false to signal that
/// discarding changes failed.
pub fn discard(locking_ranges: ModelRc<ui::LockingRange>) -> bool {
    if let Some(diff_model) = locking_ranges.as_any().downcast_ref::<DiffModel<ui::LockingRange>>() {
        diff_model.discard();
        true
    } else {
        false
    }
}
