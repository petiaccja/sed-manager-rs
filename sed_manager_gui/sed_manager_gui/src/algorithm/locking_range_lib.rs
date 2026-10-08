use core::cmp::max;
use std::ops::Range;

use sed_manager_gui_slint as ui;
use slint::{Model, ModelRc};

use crate::diff_model::DiffModel;

/// The fraction of the locking range stripe that is split evenly among all
/// locking ranges instead of proportionally to their size. This factor gives
/// tiny ranges a disproportionate size so that they are visible and have space
/// for their label.
const STRIPE_FIXED_FRACTION: f32 = 0.3;

/// Returns the normalized x position (0 .. 1.0) of a locking range as it
/// appears on the locking range stripe.
pub fn stripe_x_normalized(index: i32, locking_ranges: ModelRc<ui::LockingRange>, geometry: ui::Geometry) -> f32 {
    let (index, sector_counts, total_sector_count) = match stripe_map_input(index, locking_ranges, geometry) {
        Some(input) => input,
        None => return 0.0,
    };
    stripe_placement_normalized(index, &sector_counts, total_sector_count).0
}

/// Returns the normalized width (0 .. 1.0) of a locking range as it
/// appears on the locking range stripe.
pub fn stripe_width_normalized(index: i32, locking_ranges: ModelRc<ui::LockingRange>, geometry: ui::Geometry) -> f32 {
    let (index, sector_counts, total_sector_count) = match stripe_map_input(index, locking_ranges, geometry) {
        Some(input) => input,
        None => return 0.0,
    };
    stripe_placement_normalized(index, &sector_counts, total_sector_count).1
}

fn stripe_map_input(
    index: i32,
    locking_ranges: ModelRc<ui::LockingRange>,
    geometry: ui::Geometry,
) -> Option<(usize, Vec<Range<u64>>, u64)> {
    for row in 0..locking_ranges.row_count() {
        locking_ranges.model_tracker().track_row_data_changes(row);
    }
    let index = (index > 1).then_some(index as usize - 1)?;
    let sector_counts: Vec<_> = locking_ranges
        .iter()
        .skip(1)
        .map(|range| (max(0, range.start.value) as u64)..(max(0, range.end.value) as u64))
        .collect();
    let total_sector_count = geometry.logical_sector_count.value as u64;
    Some((index, sector_counts, total_sector_count))
}

pub fn stripe_placement_normalized(index: usize, ranges: &[Range<u64>], total_sector_count: u64) -> (f32, f32) {
    let (start, length) = match ranges.get(index).cloned() {
        Some(Range { start, end }) if start < end => (start, end - start),
        _ => return (0.0, 0.0),
    };
    let total_sector_count = f32::max(1.0, total_sector_count as f32);
    let num_non_empty = ranges.iter().filter(|s| s.start < s.end).count() as f32;
    let num_non_empty_before = ranges.iter().filter(|s| s.start < s.end && s.start < start).count() as f32;
    let (start, length) = (start as f32, length as f32);

    let x = num_non_empty_before * STRIPE_FIXED_FRACTION / num_non_empty
        + (1.0 - STRIPE_FIXED_FRACTION) * start / total_sector_count;
    let width = STRIPE_FIXED_FRACTION / num_non_empty + (1.0 - STRIPE_FIXED_FRACTION) * length / total_sector_count;
    (x, width)
}

/// Returns whether the contents of the locking ranges have changed compared
/// to the initial contents.
///
/// # Errors
///
/// If the model is not tracking changes, it's always considered dirty and
/// true is returned.
pub fn is_dirty(locking_ranges: ModelRc<ui::LockingRange>) -> bool {
    if let Some(diff_model) = locking_ranges.as_any().downcast_ref::<DiffModel<ui::LockingRange>>() {
        diff_model.is_dirty()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[expect(clippy::single_range_in_vec_init)]
    fn placement_single_range_at_start() {
        let (x, width) = stripe_placement_normalized(0, &[0..20], 100);
        assert_eq!(x, 0.0);
        assert_eq!(width, 0.44);
    }

    #[test]
    #[expect(clippy::single_range_in_vec_init)]
    fn placement_single_range_at_end() {
        let (x, width) = stripe_placement_normalized(0, &[80..100], 100);
        assert_eq!(x, 0.56);
        assert_eq!(width, 0.44);
    }
}
