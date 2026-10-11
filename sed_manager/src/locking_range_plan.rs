use sed_spec::objects::LockingRange;
use tracing::warn;

use crate::Alignment;




///-----------------------------------------------------------------------------
/// PREVIOUS ATTEMPTS
///-----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LockingRangeUpdateError {
    #[error("The actual state of the locking ranges does not match the expected state")]
    ActualNotMatchingExpected,
    #[error(
        "The start and length of one or more ranges is not available; nor the device nor the desired state contains the span"
    )]
    MissingRangeSpan,
}

/// Check if the expected locking ranges match the actual ones.
///
/// In a simplistic view, this checks if the two sets of locking ranges are "the
/// same". It's a bit more nuanced in practice:
/// - The ordering of the ranges does not matter.
/// - Fields not specified (i.e. [`None`]) in the `expected` locking ranges are
///   ignored for comparison, even if they are specified in the `actual` locking
///   range.
pub fn expected_matches_actual(expected: &[LockingRange], actual: &[LockingRange]) -> bool {
    let mut expected_sorted: Vec<_> = expected.iter().collect();
    expected_sorted.sort_by_key(|locking_range| locking_range.uid);
    let mut actual_sorted: Vec<_> = actual.iter().collect();
    actual_sorted.sort_by_key(|locking_range| locking_range.uid);

    if expected_sorted.len() != actual_sorted.len() {
        return false;
    }

    expected_sorted
        .into_iter()
        .zip(actual_sorted.into_iter())
        .all(|(expected, actual)| expected_matches_actual_object(expected, actual))
}

/// Check if the `expected` locking range matches the `actual` based on the
/// rules explained in [`matches_actual`].
fn expected_matches_actual_object(expected: &LockingRange, actual: &LockingRange) -> bool {
    if !expected.uid.is_some() {
        warn!("expected locking range is missing the UID");
        return false;
    }
    if !actual.uid.is_some() {
        warn!("expected locking range is missing the UID");
        return false;
    }

    expected.uid == actual.uid
        && expected_matches_actual_field(expected.common_name.as_ref(), actual.common_name.as_ref())
        && expected_matches_actual_field(expected.range_start.as_ref(), actual.range_start.as_ref())
        && expected_matches_actual_field(expected.range_length.as_ref(), actual.range_length.as_ref())
        && expected_matches_actual_field(expected.read_lock_enabled.as_ref(), actual.read_lock_enabled.as_ref())
        && expected_matches_actual_field(expected.write_lock_enabled.as_ref(), actual.write_lock_enabled.as_ref())
        && expected_matches_actual_field(expected.read_locked.as_ref(), actual.read_locked.as_ref())
        && expected_matches_actual_field(expected.write_locked.as_ref(), actual.write_locked.as_ref())
        && expected_matches_actual_field(expected.lock_on_reset.as_ref(), actual.lock_on_reset.as_ref())
        && expected_matches_actual_field(expected.active_key.as_ref(), actual.active_key.as_ref())
        && expected_matches_actual_field(expected.next_key.as_ref(), actual.next_key.as_ref())
        && expected_matches_actual_field(expected.reencrypt_state.as_ref(), actual.reencrypt_state.as_ref())
        && expected_matches_actual_field(expected.reencrypt_request.as_ref(), actual.reencrypt_request.as_ref())
        && expected_matches_actual_field(expected.adv_key_mode.as_ref(), actual.adv_key_mode.as_ref())
        && expected_matches_actual_field(expected.verify_mode.as_ref(), actual.verify_mode.as_ref())
        && expected_matches_actual_field(expected.cont_on_reset.as_ref(), actual.cont_on_reset.as_ref())
        && expected_matches_actual_field(expected.last_reencrypt_lba.as_ref(), actual.last_reencrypt_lba.as_ref())
        && expected_matches_actual_field(expected.last_reenc_stat.as_ref(), actual.last_reenc_stat.as_ref())
        && expected_matches_actual_field(expected.general_status.as_ref(), actual.general_status.as_ref())
}

fn expected_matches_actual_field<T>(expected: Option<&T>, actual: Option<&T>) -> bool
where
    T: PartialEq,
{
    if let Some(expected) = expected { Some(expected) == actual } else { true }
}

/// Check if the locking ranges are accepted by the device:
/// - They must have the proper alignment
/// - They must not exceed the device's size
/// - They must not overlap
///
/// # Errors
///
/// [`None`] is returned if any of the ranges are missing the `range_start` and
/// `range_length` values. In that case, correctness cannot be determined.
pub fn check_correctness(locking_ranges: &[LockingRange], alignment: &Alignment, capacity: u64) -> Option<bool> {
    Some(
        try_all(locking_ranges.iter().map(|locking_range| is_aligned(locking_range, alignment)))?
            && try_all(locking_ranges.iter().map(|locking_range| is_within_capacity(locking_range, capacity)))?
            && !is_overlapping(locking_ranges)?,
    )
}

fn is_aligned(locking_range: &LockingRange, alignment: &Alignment) -> Option<bool> {
    if !alignment.alignment_required {
        return Some(true);
    }
    let start = locking_range.range_start?;
    let length = locking_range.range_length?;

    let is_start_aligned = start == alignment.align(start);
    let is_length_aligned = !alignment.alignment_required || length.is_multiple_of(alignment.alignment_granularity);
    Some(is_start_aligned && is_length_aligned)
}

fn is_within_capacity(locking_range: &LockingRange, capacity: u64) -> Option<bool> {
    let end = locking_range.range_start? + locking_range.range_length?;
    Some(end <= capacity)
}

fn is_overlapping(locking_range: &[LockingRange]) -> Option<bool> {
    let mut range_spans = locking_range
        .iter()
        .map(|LockingRange { range_start, range_length, .. }| match (range_start, range_length) {
            (Some(start), Some(length)) => Some(*start..(start + length)),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;

    range_spans.sort_by_key(|span| span.start);
    Some(range_spans.windows(2).all(|window| window[0].end > window[1].start))
}

/// Returns:
/// - `Some(true)` if ALL values are present and `true`.
/// - `Some(false)` if ALL values are present BUT ANY of them are `false`.
/// - `None` if ANY of the values are missing.
fn try_all(values: impl IntoIterator<Item = Option<bool>>) -> Option<bool> {
    let mut result = true;
    for value in values {
        result = result && value?;
    }
    Some(result)
}

/// If the `desired` ranges don't have the `range_start` and `range_end` values
/// for some of the ranges, they are filled in from the `actual`.
pub fn backfill_incomplete_spans(desired: &mut [LockingRange], actual: &[LockingRange]) {
    for (index, desired) in desired.iter_mut().enumerate() {
        if let Some(matching_actual) = actual
            .get(index)
            .and_then(|actual| (actual.uid == desired.uid).then_some(actual))
            .or_else(|| actual.iter().find(|actual| actual.uid == desired.uid))
        {
            desired.range_start = desired.range_start.or(matching_actual.range_start);
            desired.range_length = desired.range_length.or(matching_actual.range_length);
        }
    }
}

pub fn create_plan(desired: &[LockingRange], actual: &[LockingRange]) {

}

#[cfg(test)]
mod tests {
    use std::num::NonZero;

    use super::*;

    use rstest::rstest;
    use sed_packet::UidRange;
    use sed_spec::objects::LockingRangeRef;

    const RANGE_UID: UidRange<LockingRangeRef> = UidRange {
        start: LockingRangeRef::from_half(NonZero::new(1).unwrap()),
        end: LockingRangeRef::from_half(NonZero::new(2048).unwrap()),
        step: 1,
    };

    #[rstest]
    #[case::equal(
        &[
            LockingRange{ uid: RANGE_UID.get(1), range_start: Some(1), range_length: Some(6), ..Default::default() },
            LockingRange{ uid: RANGE_UID.get(2), range_start: Some(83), range_length: Some(23), ..Default::default() },
        ],
        &[
            LockingRange{ uid: RANGE_UID.get(1), range_start: Some(1), range_length: Some(6), ..Default::default() },
            LockingRange{ uid: RANGE_UID.get(2), range_start: Some(83), range_length: Some(23), ..Default::default() },
        ],
        true
    )]
    #[case::not_equal(
        &[
            LockingRange{ uid: RANGE_UID.get(1),range_start: Some(1), range_length: Some(6), ..Default::default() },
            LockingRange{ uid: RANGE_UID.get(2),range_start: Some(94), range_length: Some(23), ..Default::default() },
        ],
        &[
            LockingRange{ uid: RANGE_UID.get(1),range_start: Some(1), range_length: Some(6), ..Default::default() },
            LockingRange{ uid: RANGE_UID.get(2),range_start: Some(83), range_length: Some(23), ..Default::default() },
        ],
        false
    )]
    #[case::unspecified(
        &[
            LockingRange{ uid: RANGE_UID.get(1),range_start: Some(1), range_length: Some(6), ..Default::default() },
            LockingRange{ uid: RANGE_UID.get(2),range_start: None, range_length: Some(23), ..Default::default() },
        ],
        &[
            LockingRange{ uid: RANGE_UID.get(1), range_start: Some(1), range_length: Some(6), ..Default::default() },
            LockingRange{ uid: RANGE_UID.get(2), range_start: Some(83), range_length: Some(23), ..Default::default() },
        ],
        true
    )]
    #[case::unordered(
        &[
            LockingRange{ uid: RANGE_UID.get(2), range_start: Some(83), range_length: Some(23), ..Default::default() },
            LockingRange{ uid: RANGE_UID.get(1), range_start: Some(1), range_length: Some(6), ..Default::default() },
        ],
        &[
            LockingRange{ uid: RANGE_UID.get(1), range_start: Some(1), range_length: Some(6), ..Default::default() },
            LockingRange{ uid: RANGE_UID.get(2), range_start: Some(83), range_length: Some(23), ..Default::default() },
        ],
        true
    )]
    #[case::no_uid(
        &[
            LockingRange{ range_start: Some(1), range_length: Some(6), ..Default::default() },
        ],
        &[
            LockingRange{ range_start: Some(1), range_length: Some(6), ..Default::default() },
        ],
        false
    )]
    fn matches_actual_(#[case] expected: &[LockingRange], #[case] actual: &[LockingRange], #[case] result: bool) {
        assert_eq!(expected_matches_actual(expected, actual), result);
    }

    #[rstest]
    #[case::both_aligned(
        LockingRange{ range_start: Some(20), range_length: Some(8), ..Default::default() },
        Alignment { alignment_required: true, alignment_granularity: 8, lowest_aligned_lba: 4 },
        Some(true),
    )]
    #[case::start_unaligned(
        LockingRange{ range_start: Some(19), range_length: Some(8), ..Default::default() },
        Alignment { alignment_required: true, alignment_granularity: 8, lowest_aligned_lba: 4 },
        Some(false),
    )]
    #[case::length_unaligned(
        LockingRange{ range_start: Some(20), range_length: Some(7), ..Default::default() },
        Alignment { alignment_required: true, alignment_granularity: 8, lowest_aligned_lba: 4 },
        Some(false),
    )]
    #[case::no_alignment_required(
        LockingRange{ range_start: Some(19), range_length: Some(7), ..Default::default() },
        Alignment { alignment_required: false, alignment_granularity: 8, lowest_aligned_lba: 4 },
        Some(true),
    )]
    #[case::unspecified_alignment_required(
        LockingRange{ range_start: None, range_length: None, ..Default::default() },
        Alignment { alignment_required: true, alignment_granularity: 8, lowest_aligned_lba: 4 },
        None,
    )]
    #[case::unspecified_no_alignment_required(
        LockingRange{ range_start: None, range_length: None, ..Default::default() },
        Alignment { alignment_required: false, alignment_granularity: 8, lowest_aligned_lba: 4 },
        Some(true),
    )]
    fn is_aligned_(#[case] range: LockingRange, #[case] alignment: Alignment, #[case] result: Option<bool>) {
        assert_eq!(is_aligned(&range, &alignment), result);
    }

    #[rstest]
    #[case::exactly_capacity(
        LockingRange{ range_start: Some(7), range_length: Some(13), ..Default::default() },
        20,
        Some(true),
    )]
    #[case::one_over_capacity(
        LockingRange{ range_start: Some(7), range_length: Some(14), ..Default::default() },
        20,
        Some(false),
    )]
    #[case::unspecified(
        LockingRange{ range_start: None, range_length: Some(14), ..Default::default() },
        20,
        None,
    )]
    fn is_within_capacity_(#[case] range: LockingRange, #[case] capacity: u64, #[case] result: Option<bool>) {
        assert_eq!(is_within_capacity(&range, capacity), result);
    }

    #[rstest]
    #[case::disjoint(
        &[
            LockingRange{ range_start: Some(20), range_length: Some(9), ..Default::default() },
            LockingRange{ range_start: Some(7), range_length: Some(13), ..Default::default() },
        ],
        Some(false),
    )]
    #[case::overlapped(
        &[
            LockingRange{ range_start: Some(20), range_length: Some(9), ..Default::default() },
            LockingRange{ range_start: Some(7), range_length: Some(14), ..Default::default() },
        ],
        Some(true),
    )]
    #[case::unspecified(
        &[
            LockingRange{ range_start: None, range_length: Some(14), ..Default::default() },
            LockingRange{ range_start: Some(9), range_length: Some(17), ..Default::default() },
        ],
        None,
    )]
    fn is_overlapping_(#[case] ranges: &[LockingRange], #[case] result: Option<bool>) {
        assert_eq!(is_overlapping(&ranges), result);
    }

    #[test]
    fn check_correctness_() {
        let locking_ranges = [
            LockingRange { range_start: Some(22), range_length: Some(8), ..Default::default() },
            LockingRange { range_start: Some(14), range_length: Some(4), ..Default::default() },
        ];
        let alignment = Alignment { alignment_required: true, alignment_granularity: 4, lowest_aligned_lba: 10 };
        let capacity = 120;

        assert_eq!(check_correctness(&locking_ranges, &alignment, capacity), Some(true));
    }

    #[rstest]
    #[case::matching_order(
        vec![
            LockingRange{ uid: RANGE_UID.get(1), range_start: None, range_length: Some(142), ..Default::default() },
            LockingRange{ uid: RANGE_UID.get(2), range_start: Some(108), range_length: None, ..Default::default() },
        ],
        &[
            LockingRange{ uid: RANGE_UID.get(1), range_start: Some(3), range_length: Some(14), ..Default::default() },
            LockingRange{ uid: RANGE_UID.get(2), range_start: Some(9), range_length: Some(17), ..Default::default() },
        ],
        &[
            LockingRange{ uid: RANGE_UID.get(1), range_start: Some(3), range_length: Some(142), ..Default::default() },
            LockingRange{ uid: RANGE_UID.get(2), range_start: Some(108), range_length: Some(17), ..Default::default() },
        ],
    )]
    #[case::unordered(
        vec![
            LockingRange{ uid: RANGE_UID.get(1), range_start: None, range_length: Some(142), ..Default::default() },
            LockingRange{ uid: RANGE_UID.get(2), range_start: Some(108), range_length: None, ..Default::default() },
        ],
        &[
            LockingRange{ uid: RANGE_UID.get(2), range_start: Some(9), range_length: Some(17), ..Default::default() },
            LockingRange{ uid: RANGE_UID.get(1), range_start: Some(3), range_length: Some(14), ..Default::default() },
        ],
        &[
            LockingRange{ uid: RANGE_UID.get(1), range_start: Some(3), range_length: Some(142), ..Default::default() },
            LockingRange{ uid: RANGE_UID.get(2), range_start: Some(108), range_length: Some(17), ..Default::default() },
        ],
    )]
    #[case::wrong_id(
        vec![
            LockingRange{ uid: RANGE_UID.get(1), range_start: None, range_length: Some(142), ..Default::default() },
            LockingRange{ uid: RANGE_UID.get(2), range_start: Some(108), range_length: None, ..Default::default() },
        ],
        &[
            LockingRange{ uid: RANGE_UID.get(4), range_start: Some(9), range_length: Some(17), ..Default::default() },
            LockingRange{ uid: RANGE_UID.get(1), range_start: Some(3), range_length: Some(14), ..Default::default() },
        ],
        &[
            LockingRange{ uid: RANGE_UID.get(1), range_start: Some(3), range_length: Some(142), ..Default::default() },
            LockingRange{ uid: RANGE_UID.get(2), range_start: Some(108), range_length: None, ..Default::default() },
        ],
    )]
    fn backfill_span_(
        #[case] mut desired: Vec<LockingRange>,
        #[case] actual: &[LockingRange],
        #[case] result: &[LockingRange],
    ) {
        backfill_incomplete_spans(&mut desired, actual);
        assert_eq!(&desired, &result);
    }
}
