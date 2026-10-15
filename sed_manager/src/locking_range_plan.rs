//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::collections::BTreeMap;

use sed_spec::{
    objects::{LockingRange, LockingRangeRef},
    preconfig::opal_2::locking::locking::GLOBAL_RANGE,
};

use crate::{
    Alignment,
    set_plan::{
        LockingRangeChange, MatchCurrent, PlanError, diff_field, fill_in_desired_values, is_matching_current_field,
        map_current_by_uid,
    },
};

#[derive(Debug)]
pub struct LockingRangePlanner {
    // BTreeMap so that the order is deterministic and so are the errors.
    locking_ranges: BTreeMap<LockingRangeRef, LockingRangeChange>,
}

impl LockingRangePlanner {
    pub fn new(
        current: impl IntoIterator<Item = LockingRange>,
        desired: impl IntoIterator<Item = LockingRange>,
    ) -> Result<Self, LockingRangePlanError> {
        let mut locking_ranges = map_current_by_uid(current)?;
        fill_in_desired_values(&mut locking_ranges, desired)?;
        check_global_range_span(&locking_ranges)?;
        fill_in_missing_spans(&mut locking_ranges)?;
        check_overlap(locking_ranges.values().map(|change| &change.desired))?;
        Ok(Self { locking_ranges })
    }

    /// Check if the desired value of all ranges respectes alignment requirements.
    pub fn check_alignment(&self, alignment: &Alignment) -> Result<(), LockingRangePlanError> {
        for (uid, LockingRangeChange { desired, .. }) in &self.locking_ranges {
            if !is_aligned(desired, alignment).expect("missing range span") {
                return Err(LockingRangePlanError::IncorrectAlignment { uid: *uid });
            }
        }
        Ok(())
    }

    /// Check if the desired value of all ranges is within the capacity of the device.
    pub fn check_within_capacity(&self, capacity: u64) -> Result<(), LockingRangePlanError> {
        for (uid, LockingRangeChange { desired, .. }) in &self.locking_ranges {
            if !is_within_capacity(desired, capacity).expect("missing range span") {
                return Err(LockingRangePlanError::ExceedsCapacity { uid: *uid });
            }
        }
        Ok(())
    }

    /// Create a list of steps, when executed in order, will change the locking
    /// ranges on the device to the desired values.
    ///
    /// The plan is always valid such that executing it should not be rejected
    /// by the device due to invalid parameters. The execution might still fail
    /// due to unforeseen communication errors.
    pub fn plan(&self) -> Vec<LockingRangeStep> {
        let mut collapse_steps = Vec::new();
        let mut set_steps = Vec::new();
        for (uid, change) in &self.locking_ranges {
            let diff = change.diff();
            // Ranges that move are collapsed first, so the parameters have to
            // be set relative to the collapsed state.
            let after_collapse = if diff.range_start.is_some() || diff.range_length.is_some() {
                collapse_steps.push(LockingRangeStep::Collapse(*uid));
                collapsed(change.current.clone())
            } else {
                change.current.clone()
            };
            let diff = LockingRangeChange { current: after_collapse, desired: change.desired.clone() }.diff();
            if diff != (LockingRange { uid: Some(*uid), ..Default::default() }) {
                set_steps.push(LockingRangeStep::Set { uid: *uid, changes: diff });
            }
        }
        collapse_steps.into_iter().chain(set_steps).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockingRangeStep {
    Collapse(LockingRangeRef),
    Set { uid: LockingRangeRef, changes: LockingRange },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LockingRangePlanError {
    #[error("The {set} range at index {index} does not specify its UID")]
    MissingUid { index: usize, set: &'static str },
    #[error("The {set} range with UID {uid} does not exist on the device")]
    NotOnDevice { uid: LockingRangeRef, set: &'static str },
    #[error("The desired ranges {first} and {second} overlap")]
    OverlappingRanges { first: LockingRangeRef, second: LockingRangeRef },
    #[error("The current state of range {uid} does not match the expected state")]
    CurrentNotMatchingExpected { uid: LockingRangeRef },
    #[error("The start and/or length of the the range {uid} can not be determined")]
    MissingRangeSpan { uid: LockingRangeRef },
    #[error("The locking range {uid} has an improper alignment")]
    IncorrectAlignment { uid: LockingRangeRef },
    #[error("The locking range {uid} exceeds the capacity of the device/namespace")]
    ExceedsCapacity { uid: LockingRangeRef },
    #[error("The desired global range specifies a non-zero start or length")]
    InvalidGlobalRangeSpan,
}

impl From<PlanError<LockingRangeRef>> for LockingRangePlanError {
    fn from(value: PlanError<LockingRangeRef>) -> Self {
        match value {
            PlanError::MissingUid { index, set } => Self::MissingUid { index, set },
            PlanError::NotOnDevice { uid, set } => Self::NotOnDevice { uid, set },
            PlanError::CurrentNotMatchingExpected { uid } => Self::CurrentNotMatchingExpected { uid },
        }
    }
}

/// If a `desired` value is missing the `range_start` or `range_end` fields,
/// they are filled in from the `current` values. This ensures that all
/// `desired` ranges specify their span and can be checked for overlaps.
fn fill_in_missing_spans(
    locking_ranges: &mut BTreeMap<LockingRangeRef, LockingRangeChange>,
) -> Result<(), LockingRangePlanError> {
    for (uid, LockingRangeChange { current, desired }) in locking_ranges {
        let uid = *uid;
        if desired.range_start.is_none() {
            desired.range_start = Some(current.range_start.ok_or(LockingRangePlanError::MissingRangeSpan { uid })?);
        }
        if desired.range_length.is_none() {
            desired.range_length = Some(current.range_length.ok_or(LockingRangePlanError::MissingRangeSpan { uid })?);
        }
    }
    Ok(())
}

/// Check if the span of the global range is specified to anything but the other
/// than `0..0` required by the spec.
fn check_global_range_span(
    locking_ranges: &BTreeMap<LockingRangeRef, LockingRangeChange>,
) -> Result<(), LockingRangePlanError> {
    if let Some(LockingRangeChange { desired: global_range, .. }) = locking_ranges.get(&GLOBAL_RANGE)
        && (global_range.range_start.unwrap_or(0) != 0 || global_range.range_length.unwrap_or(0) != 0)
    {
        Err(LockingRangePlanError::InvalidGlobalRangeSpan)
    } else {
        Ok(())
    }
}

/// Returns an error if any of the `desired` locking ranges overlap.
///
/// # Panics
///
/// If any of the ranges don't have their `range_start` or `range_length` specified.
fn check_overlap<'a>(locking_ranges: impl IntoIterator<Item = &'a LockingRange>) -> Result<(), LockingRangePlanError> {
    // Zero-length ranges are filtered out because they are never considered
    // overlapping with another as per the specification.
    let mut entries: Vec<_> =
        locking_ranges.into_iter().filter(|locking_range| locking_range.range_length != Some(0)).collect();
    entries.sort_by_key(|locking_range| locking_range.range_start.expect("missing range span"));
    for window in entries.windows(2) {
        let first = window[0];
        let second = window[1];
        let first_end = first
            .range_start
            .expect("missing range span")
            .saturating_add(first.range_length.expect("missing range span"));
        let second_start = second.range_start.expect("missing range span");
        if !(first_end <= second_start) {
            return Err(LockingRangePlanError::OverlappingRanges {
                first: first.uid.unwrap_or(LockingRangeRef::null()),
                second: second.uid.unwrap_or(LockingRangeRef::null()),
            });
        }
    }

    Ok(())
}

fn is_aligned(locking_range: &LockingRange, alignment: &Alignment) -> Option<bool> {
    if !alignment.alignment_required {
        return Some(true);
    }
    let start = locking_range.range_start?;
    let length = locking_range.range_length?;
    let end = start.saturating_add(length);

    let is_start_aligned = alignment.is_lba_aligned(start);
    let is_end_aligned = alignment.is_lba_aligned(end);
    Some(is_start_aligned && is_end_aligned)
}

fn is_within_capacity(locking_range: &LockingRange, capacity: u64) -> Option<bool> {
    let end = locking_range.range_start?.saturating_add(locking_range.range_length?);
    Some(end <= capacity)
}

fn collapsed(locking_range: LockingRange) -> LockingRange {
    LockingRange { range_length: Some(0), ..locking_range }
}

impl LockingRangeChange {
    /// Returns the differences between the current and the desired value of the
    /// locking range.
    ///
    /// All fields that are unchanged are set to [`None`] in the
    /// result. The UID is always filled.
    pub fn diff(&self) -> LockingRange {
        let Self { current, desired } = self;
        LockingRange {
            uid: self.desired.uid, // UID is read-only so no diff, include the target object's UID.
            name: diff_field(current.name.as_ref(), desired.name.as_ref()).cloned(),
            common_name: diff_field(current.common_name.as_ref(), desired.common_name.as_ref()).cloned(),
            range_start: diff_field(current.range_start.as_ref(), desired.range_start.as_ref()).cloned(),
            range_length: diff_field(current.range_length.as_ref(), desired.range_length.as_ref()).cloned(),
            read_lock_enabled: diff_field(current.read_lock_enabled.as_ref(), desired.read_lock_enabled.as_ref())
                .cloned(),
            write_lock_enabled: diff_field(current.write_lock_enabled.as_ref(), desired.write_lock_enabled.as_ref())
                .cloned(),
            read_locked: diff_field(current.read_locked.as_ref(), desired.read_locked.as_ref()).cloned(),
            write_locked: diff_field(current.write_locked.as_ref(), desired.write_locked.as_ref()).cloned(),
            lock_on_reset: diff_field(current.lock_on_reset.as_ref(), desired.lock_on_reset.as_ref()).cloned(),
            active_key: diff_field(current.active_key.as_ref(), desired.active_key.as_ref()).cloned(),
            next_key: diff_field(current.next_key.as_ref(), desired.next_key.as_ref()).cloned(),
            reencrypt_state: diff_field(current.reencrypt_state.as_ref(), desired.reencrypt_state.as_ref()).cloned(),
            reencrypt_request: diff_field(current.reencrypt_request.as_ref(), desired.reencrypt_request.as_ref())
                .cloned(),
            adv_key_mode: diff_field(current.adv_key_mode.as_ref(), desired.adv_key_mode.as_ref()).cloned(),
            verify_mode: diff_field(current.verify_mode.as_ref(), desired.verify_mode.as_ref()).cloned(),
            cont_on_reset: diff_field(current.cont_on_reset.as_ref(), desired.cont_on_reset.as_ref()).cloned(),
            last_reencrypt_lba: diff_field(current.last_reencrypt_lba.as_ref(), desired.last_reencrypt_lba.as_ref())
                .cloned(),
            last_reenc_stat: diff_field(current.last_reenc_stat.as_ref(), desired.last_reenc_stat.as_ref()).cloned(),
            general_status: diff_field(current.general_status.as_ref(), desired.general_status.as_ref()).cloned(),
        }
    }
}

impl MatchCurrent for LockingRange {
    fn is_matching_current(&self, current: &LockingRange) -> bool {
        is_matching_current_field(self.uid.as_ref(), current.uid.as_ref())
            && is_matching_current_field(self.name.as_ref(), current.name.as_ref())
            && is_matching_current_field(self.common_name.as_ref(), current.common_name.as_ref())
            && is_matching_current_field(self.range_start.as_ref(), current.range_start.as_ref())
            && is_matching_current_field(self.range_length.as_ref(), current.range_length.as_ref())
            && is_matching_current_field(self.read_lock_enabled.as_ref(), current.read_lock_enabled.as_ref())
            && is_matching_current_field(self.write_lock_enabled.as_ref(), current.write_lock_enabled.as_ref())
            && is_matching_current_field(self.read_locked.as_ref(), current.read_locked.as_ref())
            && is_matching_current_field(self.write_locked.as_ref(), current.write_locked.as_ref())
            && is_matching_current_field(self.lock_on_reset.as_ref(), current.lock_on_reset.as_ref())
            && is_matching_current_field(self.active_key.as_ref(), current.active_key.as_ref())
            && is_matching_current_field(self.next_key.as_ref(), current.next_key.as_ref())
            && is_matching_current_field(self.reencrypt_state.as_ref(), current.reencrypt_state.as_ref())
            && is_matching_current_field(self.reencrypt_request.as_ref(), current.reencrypt_request.as_ref())
            && is_matching_current_field(self.adv_key_mode.as_ref(), current.adv_key_mode.as_ref())
            && is_matching_current_field(self.verify_mode.as_ref(), current.verify_mode.as_ref())
            && is_matching_current_field(self.cont_on_reset.as_ref(), current.cont_on_reset.as_ref())
            && is_matching_current_field(self.last_reencrypt_lba.as_ref(), current.last_reencrypt_lba.as_ref())
            && is_matching_current_field(self.last_reenc_stat.as_ref(), current.last_reenc_stat.as_ref())
            && is_matching_current_field(self.general_status.as_ref(), current.general_status.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZero;

    use super::*;
    use crate::set_plan::check_matching_current_all;

    use googletest::{assert_that, matchers::*};
    use rstest::rstest;
    use sed_packet::UidRange;
    use sed_spec::objects::LockingRangeRef;

    const RANGE_UID: UidRange<LockingRangeRef> = UidRange {
        start: LockingRangeRef::from_half(NonZero::new(1).unwrap()),
        end: LockingRangeRef::from_half(NonZero::new(2048).unwrap()),
        step: 1,
    };

    fn make_range(index: usize, start: u64, length: u64) -> LockingRange {
        LockingRange {
            uid: RANGE_UID.get(index),
            range_start: Some(start),
            range_length: Some(length),
            ..Default::default()
        }
    }

    /// The Global Range, a configured range, and an unconfigured (`0..0`) range.
    fn make_current_ranges() -> Vec<LockingRange> {
        vec![
            make_range(0, 0, 0),
            make_range(1, 12, 16),
            make_range(2, 0, 0),
        ]
    }

    //--------------------------------------------------------------------------
    // LockingRangePlanner
    //--------------------------------------------------------------------------

    #[test]
    fn planner_zero_length_range_inside_other() {
        let desired = [make_range(2, 20, 0)];
        assert_that!(LockingRangePlanner::new(make_current_ranges(), desired), ok(anything()));
    }

    #[test]
    fn planner_untouched_zero_length_range_inside_other() {
        let current = [
            make_range(0, 0, 0),
            make_range(1, 12, 16),
            make_range(2, 20, 0),
        ];
        assert_that!(LockingRangePlanner::new(current, []), ok(anything()));
    }

    #[test]
    fn planner_overlap() {
        let desired = [make_range(2, 20, 16)];
        assert_that!(
            LockingRangePlanner::new(make_current_ranges(), desired),
            err(eq(&LockingRangePlanError::OverlappingRanges {
                first: RANGE_UID.get(1).unwrap(),
                second: RANGE_UID.get(2).unwrap()
            }))
        );
    }

    #[rstest]
    #[case::lowest_aligned_lba_zero(Alignment { alignment_required: true, alignment_granularity: 4, lowest_aligned_lba: 0 })]
    #[case::lowest_aligned_lba_non_zero(Alignment { alignment_required: true, alignment_granularity: 8, lowest_aligned_lba: 4 })]
    fn planner_check_alignment_untouched_global_range(#[case] alignment: Alignment) {
        // The Global Range and the unconfigured range both span `0..0`, which
        // is not aligned to the lowest aligned LBA, but must still be accepted.
        let planner = LockingRangePlanner::new(make_current_ranges(), []).unwrap();
        assert_that!(planner.check_alignment(&alignment), ok(eq(&())));
    }

    #[test]
    fn planner_check_alignment_unaligned() {
        let alignment = Alignment { alignment_required: true, alignment_granularity: 8, lowest_aligned_lba: 4 };
        let planner = LockingRangePlanner::new(make_current_ranges(), [make_range(2, 36, 7)]).unwrap();
        assert_that!(
            planner.check_alignment(&alignment),
            err(eq(&LockingRangePlanError::IncorrectAlignment { uid: RANGE_UID.get(2).unwrap() }))
        );
    }

    #[test]
    fn planner_check_within_capacity_overflowing_span() {
        let planner = LockingRangePlanner::new(make_current_ranges(), [make_range(2, 100, u64::MAX)]).unwrap();
        assert_that!(
            planner.check_within_capacity(1000),
            err(eq(&LockingRangePlanError::ExceedsCapacity { uid: RANGE_UID.get(2).unwrap() }))
        );
    }

    #[test]
    fn planner_overflowing_span_overlap() {
        let desired = [make_range(1, 0, u64::MAX), make_range(2, 100, 16)];
        assert_that!(
            LockingRangePlanner::new(make_current_ranges(), desired),
            err(eq(&LockingRangePlanError::OverlappingRanges {
                first: RANGE_UID.get(1).unwrap(),
                second: RANGE_UID.get(2).unwrap()
            }))
        );
    }

    fn make_collapse_step(index: usize) -> LockingRangeStep {
        LockingRangeStep::Collapse(RANGE_UID.get(index).unwrap())
    }

    fn make_set_step(changes: LockingRange) -> LockingRangeStep {
        LockingRangeStep::Set { uid: changes.uid.unwrap(), changes }
    }

    #[test]
    fn planner_plan_no_changes() {
        let planner = LockingRangePlanner::new(make_current_ranges(), []).unwrap();
        assert_that!(planner.plan(), is_empty());
    }

    #[test]
    fn planner_plan_without_moving() {
        let desired = [LockingRange { read_locked: Some(true), ..make_uid_only_range(1) }];
        let planner = LockingRangePlanner::new(make_current_ranges(), desired.clone()).unwrap();
        assert_that!(planner.plan(), elements_are![eq(&make_set_step(desired[0].clone()))]);
    }

    #[test]
    fn planner_plan_moving_start() {
        // The length must be set again, as it's cleared by the collapse.
        let planner = LockingRangePlanner::new(make_current_ranges(), [make_range(1, 20, 16)]).unwrap();
        assert_that!(
            planner.plan(),
            elements_are![
                eq(&make_collapse_step(1)),
                eq(&make_set_step(make_range(1, 20, 16)))
            ]
        );
    }

    #[test]
    fn planner_plan_moving_end() {
        // The start is unaffected by the collapse, so it must not be set again.
        let planner = LockingRangePlanner::new(make_current_ranges(), [make_range(1, 12, 8)]).unwrap();
        let expected = LockingRange { range_length: Some(8), ..make_uid_only_range(1) };
        assert_that!(planner.plan(), elements_are![eq(&make_collapse_step(1)), eq(&make_set_step(expected))]);
    }

    #[test]
    fn planner_plan_collapses_before_sets() {
        // Range 2 moves into the space previously occupied by range 1.
        let desired = [make_range(1, 40, 16), make_range(2, 12, 16)];
        let planner = LockingRangePlanner::new(make_current_ranges(), desired).unwrap();
        assert_that!(
            planner.plan(),
            elements_are![
                eq(&make_collapse_step(1)),
                eq(&make_collapse_step(2)),
                eq(&make_set_step(make_range(1, 40, 16))),
                eq(&make_set_step(make_range(2, 12, 16)))
            ]
        );
    }

    //--------------------------------------------------------------------------
    // fill_in_missing_spans
    //--------------------------------------------------------------------------

    #[test]
    fn fill_in_missing_spans_() {
        let mut input = [
            (
                RANGE_UID.get(1).unwrap(),
                LockingRangeChange {
                    current: LockingRange {
                        uid: RANGE_UID.get(1),
                        range_start: Some(1),
                        range_length: Some(2),
                        ..Default::default()
                    },
                    desired: LockingRange {
                        uid: RANGE_UID.get(1),
                        range_start: None,
                        range_length: None,
                        ..Default::default()
                    },
                },
            ),
            (
                RANGE_UID.get(2).unwrap(),
                LockingRangeChange {
                    current: LockingRange {
                        uid: RANGE_UID.get(2),
                        range_start: Some(1),
                        range_length: Some(2),
                        ..Default::default()
                    },
                    desired: LockingRange {
                        uid: RANGE_UID.get(2),
                        range_start: Some(3),
                        range_length: Some(4),
                        ..Default::default()
                    },
                },
            ),
        ]
        .into();

        let expected = [
            (
                RANGE_UID.get(1).unwrap(),
                LockingRangeChange {
                    current: LockingRange {
                        uid: RANGE_UID.get(1),
                        range_start: Some(1),
                        range_length: Some(2),
                        ..Default::default()
                    },
                    desired: LockingRange {
                        uid: RANGE_UID.get(1),
                        range_start: Some(1),
                        range_length: Some(2),
                        ..Default::default()
                    },
                },
            ),
            (
                RANGE_UID.get(2).unwrap(),
                LockingRangeChange {
                    current: LockingRange {
                        uid: RANGE_UID.get(2),
                        range_start: Some(1),
                        range_length: Some(2),
                        ..Default::default()
                    },
                    desired: LockingRange {
                        uid: RANGE_UID.get(2),
                        range_start: Some(3),
                        range_length: Some(4),
                        ..Default::default()
                    },
                },
            ),
        ]
        .into();
        fill_in_missing_spans(&mut input).unwrap();
        assert_that!(input, eq(&expected));
    }

    //--------------------------------------------------------------------------
    // check_overlap
    //--------------------------------------------------------------------------

    #[rstest]
    #[case::non_overlapping_ordered(
        vec![
            LockingRange{ uid: RANGE_UID.get(1), range_start: Some(10), range_length: Some(30), ..Default::default() },
            LockingRange{ uid: RANGE_UID.get(2), range_start: Some(40), range_length: Some(50), ..Default::default() },
        ],
        Ok::<_, LockingRangePlanError>(()),
    )]
    #[case::non_overlapping_reversed(
        vec![
            LockingRange{ uid: RANGE_UID.get(2), range_start: Some(40), range_length: Some(50), ..Default::default() },
            LockingRange{ uid: RANGE_UID.get(1), range_start: Some(10), range_length: Some(30), ..Default::default() },
        ],
        Ok::<_, LockingRangePlanError>(()),
    )]
    #[case::overlapping(
        vec![
            LockingRange{ uid: RANGE_UID.get(2), range_start: Some(39), range_length: Some(50), ..Default::default() },
            LockingRange{ uid: RANGE_UID.get(1), range_start: Some(10), range_length: Some(30), ..Default::default() },
        ],
        Err::<(),_>( LockingRangePlanError::OverlappingRanges { first: RANGE_UID.get(1).unwrap(), second: RANGE_UID.get(2).unwrap() }),
    )]
    #[case::touching(
        vec![
            LockingRange{ uid: RANGE_UID.get(1), range_start: Some(10), range_length: Some(30), ..Default::default() },
            LockingRange{ uid: RANGE_UID.get(2), range_start: Some(40), range_length: Some(50), ..Default::default() },
        ],
        Ok::<_, LockingRangePlanError>(()),
    )]
    #[case::zero_length_inside_other(
        vec![
            LockingRange{ uid: RANGE_UID.get(1), range_start: Some(10), range_length: Some(80), ..Default::default() },
            LockingRange{ uid: RANGE_UID.get(2), range_start: Some(50), range_length: Some(0), ..Default::default() },
        ],
        Ok::<_, LockingRangePlanError>(()),
    )]
    #[case::zero_length_same_start(
        vec![
            LockingRange{ uid: RANGE_UID.get(1), range_start: Some(10), range_length: Some(0), ..Default::default() },
            LockingRange{ uid: RANGE_UID.get(2), range_start: Some(10), range_length: Some(30), ..Default::default() },
            LockingRange{ uid: RANGE_UID.get(3), range_start: Some(10), range_length: Some(0), ..Default::default() },
        ],
        Ok::<_, LockingRangePlanError>(()),
    )]
    #[case::non_adjacent_overlap(
        vec![
            LockingRange{ uid: RANGE_UID.get(1), range_start: Some(0), range_length: Some(100), ..Default::default() },
            LockingRange{ uid: RANGE_UID.get(2), range_start: Some(10), range_length: Some(0), ..Default::default() },
            LockingRange{ uid: RANGE_UID.get(3), range_start: Some(50), range_length: Some(10), ..Default::default() },
        ],
        Err::<(),_>( LockingRangePlanError::OverlappingRanges { first: RANGE_UID.get(1).unwrap(), second: RANGE_UID.get(3).unwrap() }),
    )]
    #[case::overflowing_span(
        vec![
            LockingRange{ uid: RANGE_UID.get(1), range_start: Some(10), range_length: Some(u64::MAX), ..Default::default() },
            LockingRange{ uid: RANGE_UID.get(2), range_start: Some(40), range_length: Some(50), ..Default::default() },
        ],
        Err::<(),_>( LockingRangePlanError::OverlappingRanges { first: RANGE_UID.get(1).unwrap(), second: RANGE_UID.get(2).unwrap() }),
    )]
    fn check_overlap_(#[case] ranges: Vec<LockingRange>, #[case] expected_result: Result<(), LockingRangePlanError>) {
        assert_that!(check_overlap(&ranges), eq(&expected_result));
    }

    //--------------------------------------------------------------------------
    // check_global_range_span
    //--------------------------------------------------------------------------

    #[rstest]
    #[case::omitted(None, None, Ok(()))]
    #[case::zero(Some(0), Some(0), Ok(()))]
    #[case::non_zero_start(Some(10), None, Err(LockingRangePlanError::InvalidGlobalRangeSpan))]
    #[case::non_zero_length(None, Some(10), Err(LockingRangePlanError::InvalidGlobalRangeSpan))]
    fn check_global_range_span_(
        #[case] range_start: Option<u64>,
        #[case] range_length: Option<u64>,
        #[case] expected_result: Result<(), LockingRangePlanError>,
    ) {
        let global_range = LockingRange { uid: Some(GLOBAL_RANGE), ..Default::default() };
        let locking_ranges = [(
            GLOBAL_RANGE,
            LockingRangeChange {
                current: global_range.clone(),
                desired: LockingRange { range_start, range_length, ..global_range },
            },
        )]
        .into();
        assert_that!(check_global_range_span(&locking_ranges), eq(&expected_result));
    }

    #[test]
    fn check_global_range_span_ignores_other_ranges() {
        let range =
            LockingRange { uid: RANGE_UID.get(1), range_start: Some(10), range_length: Some(10), ..Default::default() };
        let locking_ranges =
            [(RANGE_UID.get(1).unwrap(), LockingRangeChange { current: range.clone(), desired: range })].into();
        assert_that!(check_global_range_span(&locking_ranges), ok(eq(&())));
    }

    //--------------------------------------------------------------------------
    // check_matching_current_all
    //--------------------------------------------------------------------------

    fn make_uid_only_range(index: usize) -> LockingRange {
        LockingRange { uid: RANGE_UID.get(index), ..Default::default() }
    }

    fn locked(range: LockingRange) -> LockingRange {
        LockingRange { read_locked: Some(true), write_locked: Some(true), ..range }
    }

    #[rstest]
    #[case::equal(make_current_ranges(), Ok(()))]
    #[case::empty(vec![], Ok(()))]
    #[case::reordered(vec![make_range(2, 0, 0), make_range(0, 0, 0), make_range(1, 12, 16)], Ok(()))]
    #[case::subset(vec![make_range(1, 12, 16)], Ok(()))]
    #[case::unspecified_fields_ignored(vec![make_uid_only_range(0), make_uid_only_range(1), make_uid_only_range(2)], Ok(()))]
    #[case::additional_field_matching(vec![locked(make_range(1, 12, 16))], Ok(()))]
    #[case::additional_field_mismatching(
        vec![locked(make_range(2, 0, 0))],
        Err(LockingRangePlanError::CurrentNotMatchingExpected { uid: RANGE_UID.get(2).unwrap() }),
    )]
    #[case::span_mismatching(
        vec![make_range(0, 0, 0), make_range(1, 12, 8)],
        Err(LockingRangePlanError::CurrentNotMatchingExpected { uid: RANGE_UID.get(1).unwrap() }),
    )]
    #[case::first_mismatch_reported(
        vec![make_range(2, 4, 4), make_range(1, 12, 8)],
        Err(LockingRangePlanError::CurrentNotMatchingExpected { uid: RANGE_UID.get(2).unwrap() }),
    )]
    #[case::missing_uid(
        vec![make_range(1, 12, 16), LockingRange { range_start: Some(0), ..Default::default() }],
        Err(LockingRangePlanError::MissingUid { index: 1, set: "expected" }),
    )]
    #[case::not_on_device(
        vec![make_range(1, 12, 16), make_range(5, 0, 0)],
        Err(LockingRangePlanError::NotOnDevice { uid: RANGE_UID.get(5).unwrap(), set: "expected" }),
    )]
    fn check_matching_current_all_(
        #[case] expected: Vec<LockingRange>,
        #[case] expected_result: Result<(), LockingRangePlanError>,
    ) {
        // Range 1 is locked on the device, but range 2 is not.
        let current = [
            make_range(0, 0, 0),
            locked(make_range(1, 12, 16)),
            make_range(2, 0, 0),
        ];
        assert_that!(
            check_matching_current_all(&expected, &current).map_err(LockingRangePlanError::from),
            eq(&expected_result)
        );
    }

    //--------------------------------------------------------------------------
    // is_matching_current
    //--------------------------------------------------------------------------
    #[rstest]
    #[case::equal(
        LockingRange{ range_start: None, range_length: Some(50), ..Default::default() },
        LockingRange{ range_start: None, range_length: Some(50), ..Default::default() },
        true,
    )]
    #[case::unspecified(
        LockingRange{ range_start: None, range_length: Some(50), ..Default::default() },
        LockingRange{ range_start: Some(10), range_length: Some(50), ..Default::default() },
        true,
    )]
    #[case::overspecified(
        LockingRange{ range_start: Some(10), range_length: Some(50), ..Default::default() },
        LockingRange{ range_start: None, range_length: Some(50), ..Default::default() },
        false,
    )]
    fn is_matching_current_(#[case] expected: LockingRange, #[case] current: LockingRange, #[case] result: bool) {
        assert_eq!(expected.is_matching_current(&current), result);
    }

    //--------------------------------------------------------------------------
    // is_aligned
    //--------------------------------------------------------------------------

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
    #[case::zero_start_with_aligned_end(
        LockingRange{ range_start: Some(0), range_length: Some(12), ..Default::default() },
        Alignment { alignment_required: true, alignment_granularity: 8, lowest_aligned_lba: 4 },
        Some(true),
    )]
    #[case::zero_start_with_unaligned_end(
        LockingRange{ range_start: Some(0), range_length: Some(8), ..Default::default() },
        Alignment { alignment_required: true, alignment_granularity: 8, lowest_aligned_lba: 4 },
        Some(false),
    )]
    #[case::zero_start_shorter_than_lowest_aligned_lba(
        LockingRange{ range_start: Some(0), range_length: Some(2), ..Default::default() },
        Alignment { alignment_required: true, alignment_granularity: 8, lowest_aligned_lba: 4 },
        Some(false),
    )]
    #[case::zero_length(
        LockingRange{ range_start: Some(20), range_length: Some(0), ..Default::default() },
        Alignment { alignment_required: true, alignment_granularity: 8, lowest_aligned_lba: 4 },
        Some(true),
    )]
    #[case::zero_span(
        LockingRange{ range_start: Some(0), range_length: Some(0), ..Default::default() },
        Alignment { alignment_required: true, alignment_granularity: 8, lowest_aligned_lba: 4 },
        Some(true),
    )]
    #[case::start_before_lowest_aligned_lba(
        LockingRange{ range_start: Some(2), range_length: Some(8), ..Default::default() },
        Alignment { alignment_required: true, alignment_granularity: 8, lowest_aligned_lba: 4 },
        Some(false),
    )]
    fn is_aligned_(#[case] range: LockingRange, #[case] alignment: Alignment, #[case] result: Option<bool>) {
        assert_eq!(is_aligned(&range, &alignment), result);
    }

    //--------------------------------------------------------------------------
    // is_within_capacity
    //--------------------------------------------------------------------------

    #[rstest]
    #[case::unspecified(
        LockingRange{ range_start: None, range_length: Some(50), ..Default::default() },
        60,
        None,
    )]
    #[case::exceeds_by_one(
        LockingRange{ range_start: Some(10), range_length: Some(51), ..Default::default() },
        60,
        Some(false),
    )]
    #[case::at_capacity(
        LockingRange{ range_start: Some(10), range_length: Some(50), ..Default::default() },
        60,
        Some(true),
    )]
    #[case::overflowing_span(
        LockingRange{ range_start: Some(10), range_length: Some(u64::MAX - 5), ..Default::default() },
        60,
        Some(false),
    )]
    fn is_within_capacity_(#[case] range: LockingRange, #[case] capacity: u64, #[case] result: Option<bool>) {
        assert_eq!(is_within_capacity(&range, capacity), result);
    }
}
