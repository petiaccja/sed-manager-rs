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

    use googletest::assert_that;
    use googletest::matchers::*;
    use rstest::rstest;
    use sed_packet::{ObjectRef, TableRef};
    use sed_spec_macros::Object;

    use super::*;

    const GIZMO_TABLE: TableRef = TableRef::new(0xF100_0000_0000_0000);

    type GizmoRef = ObjectRef<{ GIZMO_TABLE.to_u64() }>;

    #[derive(Debug, Clone, Default, PartialEq, Eq, Object)]
    #[object(table=GIZMO_TABLE)]
    struct Gizmo {
        uid: Option<GizmoRef>,
        foo: Option<u64>,
        bar: Option<String>,
    }

    impl ObjectUid for Gizmo {
        fn uid(&self) -> Option<Self::Ref> {
            self.uid
        }

        fn set_uid(&mut self, uid: Option<Self::Ref>) {
            self.uid = uid;
        }
    }

    impl MatchCurrent for Gizmo {
        fn is_matching_current(&self, current: &Self) -> bool {
            is_matching_current_field(self.uid.as_ref(), current.uid.as_ref())
                && is_matching_current_field(self.foo.as_ref(), current.foo.as_ref())
                && is_matching_current_field(self.bar.as_ref(), current.bar.as_ref())
        }
    }

    fn gizmo_uid(index: u32) -> GizmoRef {
        GizmoRef::from_half(NonZero::new(index).unwrap())
    }

    fn make_uid_only_gizmo(index: u32) -> Gizmo {
        Gizmo { uid: Some(gizmo_uid(index)), ..Default::default() }
    }

    fn make_gizmo(index: u32, foo: u64) -> Gizmo {
        Gizmo { foo: Some(foo), ..make_uid_only_gizmo(index) }
    }

    fn with_bar(gizmo: Gizmo, bar: &str) -> Gizmo {
        Gizmo { bar: Some(bar.into()), ..gizmo }
    }

    //--------------------------------------------------------------------------
    // fill_in_desired_values
    //--------------------------------------------------------------------------

    #[rstest]
    #[case::success(
        &[1, 2],
        &[make_gizmo(1, 10)],
        Ok::<_, PlanError<_>>([make_gizmo(1, 10), make_uid_only_gizmo(2)]),
    )]
    #[case::missing_uid(
        &[1],
        &[Gizmo { foo: Some(10), ..Default::default() }],
        Err::<[_; 0], _>(PlanError::MissingUid { index: 0, set: "desired" }),
    )]
    #[case::not_on_device(
        &[1],
        &[make_gizmo(34, 10)],
        Err::<[_; 0], _>(PlanError::NotOnDevice { uid: gizmo_uid(34), set: "desired" }),
    )]
    fn fill_in_desired_values_(
        #[case] indices: &[u32],
        #[case] desired: &[Gizmo],
        #[case] expected_result: Result<impl IntoIterator<Item = Gizmo>, PlanError<GizmoRef>>,
    ) {
        let mut gizmos = map_current_by_uid(indices.iter().map(|index| make_uid_only_gizmo(*index))).unwrap();

        let actual_result = fill_in_desired_values(&mut gizmos, desired.iter().cloned());
        match expected_result {
            Ok(expected_desired) => {
                let mut expected_desired: Vec<_> = expected_desired.into_iter().collect();
                expected_desired.sort_by_key(|gizmo| gizmo.uid);
                let mut actual_desired: Vec<_> = gizmos.values().map(|change| change.desired.clone()).collect();
                actual_desired.sort_by_key(|gizmo| gizmo.uid);
                assert_that!(actual_desired, eq(&expected_desired));
            }
            Err(expected_err) => assert_that!(actual_result, err(eq(&expected_err))),
        };
    }

    //--------------------------------------------------------------------------
    // diff_field
    //--------------------------------------------------------------------------

    #[rstest]
    #[case::both_unspecified(None, None, None)]
    #[case::desired_unspecified(Some(10), None, None)]
    #[case::current_unspecified(None, Some(10), Some(10))]
    #[case::equal(Some(10), Some(10), None)]
    #[case::different(Some(10), Some(20), Some(20))]
    #[case::changed_to_zero(Some(10), Some(0), Some(0))]
    fn diff_field_(#[case] current: Option<u64>, #[case] desired: Option<u64>, #[case] result: Option<u64>) {
        assert_eq!(diff_field(current.as_ref(), desired.as_ref()), result.as_ref());
    }

    #[test]
    fn diff_field_returns_desired() {
        let current = String::from("current");
        let desired = String::from("desired");
        let result = diff_field(Some(&current), Some(&desired));
        assert!(result.is_some_and(|result| std::ptr::eq(result, &desired)));
    }

    //--------------------------------------------------------------------------
    // check_matching_current_all
    //--------------------------------------------------------------------------

    /// Only gizmo 2 has `bar` specified on the device.
    fn make_current_gizmos() -> Vec<Gizmo> {
        vec![
            make_gizmo(1, 10),
            with_bar(make_gizmo(2, 20), "bar"),
            make_gizmo(3, 30),
        ]
    }

    #[rstest]
    #[case::equal(make_current_gizmos(), Ok(()))]
    #[case::empty(vec![], Ok(()))]
    #[case::reordered(vec![make_gizmo(3, 30), make_gizmo(1, 10), with_bar(make_gizmo(2, 20), "bar")], Ok(()))]
    #[case::subset(vec![make_gizmo(1, 10)], Ok(()))]
    #[case::unspecified_fields_ignored(vec![make_uid_only_gizmo(1), make_uid_only_gizmo(2), make_uid_only_gizmo(3)], Ok(()))]
    #[case::additional_field_matching(vec![with_bar(make_gizmo(2, 20), "bar")], Ok(()))]
    #[case::additional_field_mismatching(
        vec![with_bar(make_gizmo(3, 30), "bar")],
        Err(PlanError::CurrentNotMatchingExpected { uid: gizmo_uid(3) }),
    )]
    #[case::field_mismatching(
        vec![make_gizmo(1, 10), make_gizmo(2, 25)],
        Err(PlanError::CurrentNotMatchingExpected { uid: gizmo_uid(2) }),
    )]
    #[case::first_mismatch_reported(
        vec![make_gizmo(3, 35), make_gizmo(2, 25)],
        Err(PlanError::CurrentNotMatchingExpected { uid: gizmo_uid(3) }),
    )]
    #[case::missing_uid(
        vec![make_gizmo(1, 10), Gizmo { foo: Some(10), ..Default::default() }],
        Err(PlanError::MissingUid { index: 1, set: "expected" }),
    )]
    #[case::not_on_device(
        vec![make_gizmo(1, 10), make_gizmo(6, 0)],
        Err(PlanError::NotOnDevice { uid: gizmo_uid(6), set: "expected" }),
    )]
    fn check_matching_current_all_(
        #[case] expected: Vec<Gizmo>,
        #[case] expected_result: Result<(), PlanError<GizmoRef>>,
    ) {
        let current = make_current_gizmos();
        assert_that!(check_matching_current_all(&expected, &current), eq(&expected_result));
    }

    #[test]
    fn check_matching_current_all_current_missing_uid() {
        let current = [
            make_gizmo(1, 10),
            Gizmo { foo: Some(20), ..Default::default() },
        ];
        assert_that!(
            check_matching_current_all(&[make_gizmo(1, 10)], &current),
            err(eq(&PlanError::MissingUid { index: 1, set: "current" }))
        );
    }
}
