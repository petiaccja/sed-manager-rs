//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::collections::BTreeMap;

use sed_packet::{Object, ObjectUid};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlanError<Ref: core::fmt::Display> {
    #[error("The {set} object at index {index} does not specify its UID")]
    MissingUid { index: usize, set: &'static str },
    #[error("The {set} object with UID {uid} does not exist on the device")]
    NotOnDevice { uid: Ref, set: &'static str },
    #[error("The current state of object {uid} does not match the expected state")]
    CurrentNotMatchingExpected { uid: Ref },
}

/// Return the value that has to be set in order to turn `current` to `desired`.
///
/// For example, if `current` and `desired` are already equal [`None`] is
/// returned.
pub fn diff_field<'a, T>(current: Option<&'a T>, desired: Option<&'a T>) -> Option<&'a T>
where
    T: PartialEq,
{
    match (current, desired) {
        (_, None) => None,
        (None, _) => desired,
        (Some(current), Some(desired)) => {
            if current == desired {
                None
            } else {
                Some(desired)
            }
        }
    }
}

/// Check if the `expected` field matches the `current` one.
///
/// If `expected` is provided, it must be equal to `current`.
/// If `expected` isn't provided, `true` is returned.
pub fn is_matching_current_field<T>(expected: Option<&T>, current: Option<&T>) -> bool
where
    T: PartialEq,
{
    if let Some(expected) = expected {
        Some(expected) == current
    } else {
        true
    }
}

/// A change in the value of the object from `current` to `desired`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectChange<O> {
    pub current: O,
    pub desired: O,
}

/// Maps the `current` objects by UID. The map of changes has the
/// `current` state of the objects filled in, but the `desired` state is left as
/// [`Default`] (except for the UID field).
///
/// # Errors
///
/// If any of the objects don't have a UID. The device should always return the UIDs.
pub fn map_current_by_uid<O>(
    current: impl IntoIterator<Item = O>,
) -> Result<BTreeMap<O::Ref, ObjectChange<O>>, PlanError<O::Ref>>
where
    O: Object + ObjectUid + Default,
    O::Ref: core::fmt::Display + Ord + Clone,
{
    current
        .into_iter()
        .enumerate()
        .map(|(index, current)| {
            let uid = current.uid().ok_or(PlanError::MissingUid { index, set: "current" })?;
            let mut desired = O::default();
            desired.set_uid(Some(uid.clone()));
            Ok((uid, ObjectChange { current, desired }))
        })
        .collect()
}

/// Update the `desired` values in the change map.
///
/// # Errors
///
/// - If any of the desired values don't set their UID field.
/// - If a desired field is not found in the map -- change requested, does not
///   exist on the device.
///
/// The `desired` values don't have to contain an entry for each object found
/// in the map.
pub fn fill_in_desired_values<O>(
    objects: &mut BTreeMap<O::Ref, ObjectChange<O>>,
    desired: impl IntoIterator<Item = O>,
) -> Result<(), PlanError<O::Ref>>
where
    O: Object + ObjectUid,
    O::Ref: core::fmt::Display + Ord,
{
    for (index, desired) in desired.into_iter().enumerate() {
        let uid = desired.uid().ok_or(PlanError::MissingUid { index, set: "desired" })?;
        let entry = objects.get_mut(&uid).ok_or(PlanError::NotOnDevice { uid, set: "desired" })?;
        entry.desired = desired;
    }
    Ok(())
}

/// Check if the `expected` objects match the `current` ones as found on the
/// device.
///
/// It checks if the two sets of objects are equal, with the following nuances:
/// - The ordering of the objects does not matter.
/// - Fields not specified (i.e. [`None`]) in the `expected` objects are
///   ignored for comparison, even if they are specified in the `current`
///   objects.
/// - Objects that are `current` but not `expected` are ignored.
pub fn check_matching_current_all<'a, O>(
    expected: impl IntoIterator<Item = &'a O>,
    current: impl IntoIterator<Item = &'a O>,
) -> Result<(), PlanError<O::Ref>>
where
    O: Object + ObjectUid + 'a + MatchCurrent,
    O::Ref: core::fmt::Display + Ord + Clone,
{
    let current = current
        .into_iter()
        .enumerate()
        .map(|(index, current)| {
            let uid = current.uid().ok_or(PlanError::MissingUid { index, set: "current" })?;
            Ok((uid, current))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;

    for (index, expected) in expected.into_iter().enumerate() {
        let uid = expected.uid().ok_or(PlanError::MissingUid { index, set: "expected" })?;
        let current = current.get(&uid).ok_or(PlanError::NotOnDevice { uid: uid.clone(), set: "expected" })?;
        if !expected.is_matching_current(current) {
            return Err(PlanError::CurrentNotMatchingExpected { uid });
        }
    }
    Ok(())
}

pub trait MatchCurrent {
    /// Check if `self` matches the `current` based on the rules explained in [`is_matching_current_field`].
    fn is_matching_current(&self, current: &Self) -> bool;
}

#[cfg(test)]
mod tests {
    use std::num::NonZero;

    use super::*;

    use googletest::{assert_that, matchers::*};
    use rstest::rstest;
    use sed_packet::UidRange;
    use sed_spec::objects::{LockingRange, LockingRangeRef};

    //--------------------------------------------------------------------------
    // fill_in_desired_values
    //--------------------------------------------------------------------------

    const RANGE_UID: UidRange<LockingRangeRef> = UidRange {
        start: LockingRangeRef::from_half(NonZero::new(1).unwrap()),
        end: LockingRangeRef::from_half(NonZero::new(2048).unwrap()),
        step: 1,
    };

    #[rstest]
    #[case::success(
        &[
            RANGE_UID.get(1).unwrap(),
        ],
        &[
            LockingRange{ uid: RANGE_UID.get(1), range_start: Some(10), ..Default::default() }
        ],
        Ok::<_, PlanError<_>>(
            [
                LockingRange{ uid: RANGE_UID.get(1), range_start: Some(10), ..Default::default() }
            ]
        ),
    )]
    #[case::missing_uid(
        &[
            RANGE_UID.get(1).unwrap(),
        ],
        &[
            LockingRange{ range_start: Some(10), ..Default::default() }
        ],
        Err::<[_; 0], _>(PlanError::MissingUid{ index: 0, set: "desired" }),
    )]
    #[case::not_on_device(
        &[
            RANGE_UID.get(1).unwrap(),
        ],
        &[
            LockingRange{ uid: RANGE_UID.get(34), range_start: Some(10), ..Default::default() }
        ],
        Err::<[_; 0], _>(PlanError::NotOnDevice { uid: RANGE_UID.get(34).unwrap(), set: "desired" }),
    )]
    fn fill_in_desired_values_(
        #[case] uids: &[LockingRangeRef],
        #[case] desired: &[LockingRange],
        #[case] expected_result: Result<impl IntoIterator<Item = LockingRange>, PlanError<LockingRangeRef>>,
    ) {
        let mut locking_ranges =
            map_current_by_uid(uids.iter().map(|uid| LockingRange { uid: Some(*uid), ..Default::default() })).unwrap();

        let actual_result = fill_in_desired_values(&mut locking_ranges, desired.iter().cloned());
        match expected_result {
            Ok(expected_desired) => {
                let mut expected_desired: Vec<_> = expected_desired.into_iter().collect();
                expected_desired.sort_by_key(|range| range.uid);
                let mut actual_desired: Vec<_> = locking_ranges.values().map(|change| change.desired.clone()).collect();
                actual_desired.sort_by_key(|range| range.uid);
                assert_that!(actual_desired, eq(&expected_desired));
            }
            Err(expected_err) => assert_that!(actual_result, err(eq(&expected_err))),
        };
    }

    //--------------------------------------------------------------------------
    // diff
    //--------------------------------------------------------------------------

    #[rstest]
    #[case::both_unspecified(None, None, None)]
    #[case::desired_unspecified(Some(10), None, None)]
    #[case::current_unspecified(None, Some(10), Some(10))]
    #[case::equal(Some(10), Some(10), None)]
    #[case::different(Some(10), Some(20), Some(20))]
    #[case::changed_to_zero(Some(10), Some(0), Some(0))]
    fn diff_(#[case] current: Option<u64>, #[case] desired: Option<u64>, #[case] result: Option<u64>) {
        assert_eq!(diff_field(current.as_ref(), desired.as_ref()), result.as_ref());
    }

    #[test]
    fn diff_returns_desired() {
        let current = String::from("current");
        let desired = String::from("desired");
        let result = diff_field(Some(&current), Some(&desired));
        assert!(result.is_some_and(|result| std::ptr::eq(result, &desired)));
    }
}
