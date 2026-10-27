//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::collections::BTreeMap;

use sed_spec::objects::{Authority, AuthorityRef};

use crate::locking_config::plan::{
    MatchCurrent, ObjectChange, PlanError, diff_field, fill_in_desired_values, is_matching_current_field,
    map_current_by_uid,
};

pub type AuthorityChange = ObjectChange<Authority>;

#[derive(Debug)]
pub struct AuthorityPlanner {
    // BTreeMap so that the order is deterministic and so are the errors.
    authorities: BTreeMap<AuthorityRef, AuthorityChange>,
}

impl AuthorityPlanner {
    pub fn new(
        current: impl IntoIterator<Item = Authority>,
        desired: impl IntoIterator<Item = Authority>,
    ) -> Result<Self, AuthorityPlanError> {
        let mut authorities = map_current_by_uid(current)?;
        fill_in_desired_values(&mut authorities, desired)?;
        Ok(Self { authorities })
    }

    /// Return a list of changes that have to be issued to the device so that we
    /// get the `desired` state of the authorities.
    pub fn plan(&self) -> Vec<AuthorityStep> {
        self.authorities
            .iter()
            .filter_map(|(uid, change)| {
                let diff = change.diff();
                if diff != (Authority { uid: Some(*uid), ..Default::default() }) {
                    Some(AuthorityStep { uid: *uid, changes: diff })
                } else {
                    None
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityStep {
    pub uid: AuthorityRef,
    pub changes: Authority,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AuthorityPlanError {
    #[error("The {set} authority at index {index} does not specify its UID")]
    MissingUid { index: usize, set: &'static str },
    #[error("The {set} authority with UID {uid} does not exist on the device")]
    NotOnDevice { uid: AuthorityRef, set: &'static str },
    #[error("The current state of authority {uid} does not match the expected state")]
    CurrentNotMatchingExpected { uid: AuthorityRef },
}

impl From<PlanError<AuthorityRef>> for AuthorityPlanError {
    fn from(value: PlanError<AuthorityRef>) -> Self {
        match value {
            PlanError::MissingUid { index, set } => Self::MissingUid { index, set },
            PlanError::NotOnDevice { uid, set } => Self::NotOnDevice { uid, set },
            PlanError::CurrentNotMatchingExpected { uid } => Self::CurrentNotMatchingExpected { uid },
        }
    }
}

impl AuthorityChange {
    /// Returns the differences between the current and the desired value of the
    /// authority.
    ///
    /// All fields that are unchanged are set to [`None`] in the
    /// result. The UID is always filled.
    pub fn diff(&self) -> Authority {
        let Self { current, desired } = self;
        Authority {
            uid: self.desired.uid, // UID is read-only so no diff, include the target object's UID.
            name: diff_field(current.name.as_ref(), desired.name.as_ref()).cloned(),
            common_name: diff_field(current.common_name.as_ref(), desired.common_name.as_ref()).cloned(),
            is_class: diff_field(current.is_class.as_ref(), desired.is_class.as_ref()).cloned(),
            class: diff_field(current.class.as_ref(), desired.class.as_ref()).cloned(),
            enabled: diff_field(current.enabled.as_ref(), desired.enabled.as_ref()).cloned(),
            secure: diff_field(current.secure.as_ref(), desired.secure.as_ref()).cloned(),
            hash_and_sign: diff_field(current.hash_and_sign.as_ref(), desired.hash_and_sign.as_ref()).cloned(),
            present_certificate: diff_field(current.present_certificate.as_ref(), desired.present_certificate.as_ref())
                .cloned(),
            operation: diff_field(current.operation.as_ref(), desired.operation.as_ref()).cloned(),
            credential: diff_field(current.credential.as_ref(), desired.credential.as_ref()).cloned(),
            response_sign: diff_field(current.response_sign.as_ref(), desired.response_sign.as_ref()).cloned(),
            response_exch: diff_field(current.response_exch.as_ref(), desired.response_exch.as_ref()).cloned(),
            clock_start: diff_field(current.clock_start.as_ref(), desired.clock_start.as_ref()).cloned(),
            clock_end: diff_field(current.clock_end.as_ref(), desired.clock_end.as_ref()).cloned(),
            limit: diff_field(current.limit.as_ref(), desired.limit.as_ref()).cloned(),
            uses: diff_field(current.uses.as_ref(), desired.uses.as_ref()).cloned(),
            log: diff_field(current.log.as_ref(), desired.log.as_ref()).cloned(),
            log_to: diff_field(current.log_to.as_ref(), desired.log_to.as_ref()).cloned(),
        }
    }
}

impl MatchCurrent for Authority {
    fn is_matching_current(&self, current: &Authority) -> bool {
        is_matching_current_field(self.uid.as_ref(), current.uid.as_ref())
            && is_matching_current_field(self.name.as_ref(), current.name.as_ref())
            && is_matching_current_field(self.common_name.as_ref(), current.common_name.as_ref())
            && is_matching_current_field(self.is_class.as_ref(), current.is_class.as_ref())
            && is_matching_current_field(self.class.as_ref(), current.class.as_ref())
            && is_matching_current_field(self.enabled.as_ref(), current.enabled.as_ref())
            && is_matching_current_field(self.secure.as_ref(), current.secure.as_ref())
            && is_matching_current_field(self.hash_and_sign.as_ref(), current.hash_and_sign.as_ref())
            && is_matching_current_field(self.present_certificate.as_ref(), current.present_certificate.as_ref())
            && is_matching_current_field(self.operation.as_ref(), current.operation.as_ref())
            && is_matching_current_field(self.credential.as_ref(), current.credential.as_ref())
            && is_matching_current_field(self.response_sign.as_ref(), current.response_sign.as_ref())
            && is_matching_current_field(self.response_exch.as_ref(), current.response_exch.as_ref())
            && is_matching_current_field(self.clock_start.as_ref(), current.clock_start.as_ref())
            && is_matching_current_field(self.clock_end.as_ref(), current.clock_end.as_ref())
            && is_matching_current_field(self.limit.as_ref(), current.limit.as_ref())
            && is_matching_current_field(self.uses.as_ref(), current.uses.as_ref())
            && is_matching_current_field(self.log.as_ref(), current.log.as_ref())
            && is_matching_current_field(self.log_to.as_ref(), current.log_to.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZero;

    use googletest::assert_that;
    use googletest::matchers::*;
    use rstest::rstest;
    use sed_packet::UidRange;

    use super::*;

    const AUTHORITY_UID: UidRange<AuthorityRef> = UidRange {
        start: AuthorityRef::from_half(NonZero::new(1).unwrap()),
        end: AuthorityRef::from_half(NonZero::new(2048).unwrap()),
        step: 1,
    };

    fn make_authority(index: usize, common_name: &str, enabled: bool) -> Authority {
        Authority {
            uid: AUTHORITY_UID.get(index),
            common_name: Some(common_name.into()),
            enabled: Some(enabled),
            ..Default::default()
        }
    }

    fn make_uid_only_authority(index: usize) -> Authority {
        Authority { uid: AUTHORITY_UID.get(index), ..Default::default() }
    }

    /// An enabled authority with a name, and two disabled ones without a name.
    fn make_current_authorities() -> Vec<Authority> {
        vec![
            make_authority(0, "Alice", true),
            make_authority(1, "", false),
            make_authority(2, "", false),
        ]
    }

    //--------------------------------------------------------------------------
    // AuthorityPlanner
    //--------------------------------------------------------------------------

    #[test]
    fn planner_current_missing_uid() {
        let current = [
            make_authority(0, "Alice", true),
            Authority { enabled: Some(false), ..Default::default() },
        ];
        assert_that!(
            AuthorityPlanner::new(current, []),
            err(eq(&AuthorityPlanError::MissingUid { index: 1, set: "current" }))
        );
    }

    #[test]
    fn planner_desired_missing_uid() {
        let desired = [Authority { enabled: Some(true), ..Default::default() }];
        assert_that!(
            AuthorityPlanner::new(make_current_authorities(), desired),
            err(eq(&AuthorityPlanError::MissingUid { index: 0, set: "desired" }))
        );
    }

    #[test]
    fn planner_desired_not_on_device() {
        let desired = [make_authority(5, "Bob", true)];
        assert_that!(
            AuthorityPlanner::new(make_current_authorities(), desired),
            err(eq(&AuthorityPlanError::NotOnDevice { uid: AUTHORITY_UID.get(5).unwrap(), set: "desired" }))
        );
    }

    fn make_step(changes: Authority) -> AuthorityStep {
        AuthorityStep { uid: changes.uid.unwrap(), changes }
    }

    #[test]
    fn planner_plan_no_changes() {
        let planner = AuthorityPlanner::new(make_current_authorities(), []).unwrap();
        assert_that!(planner.plan(), is_empty());
    }

    #[test]
    fn planner_plan_unchanged_desired() {
        let planner = AuthorityPlanner::new(make_current_authorities(), make_current_authorities()).unwrap();
        assert_that!(planner.plan(), is_empty());
    }

    #[test]
    fn planner_plan_single_field() {
        let desired = [Authority { enabled: Some(true), ..make_uid_only_authority(1) }];
        let planner = AuthorityPlanner::new(make_current_authorities(), desired.clone()).unwrap();
        assert_that!(planner.plan(), elements_are![eq(&make_step(desired[0].clone()))]);
    }

    #[test]
    fn planner_plan_unchanged_fields_omitted() {
        // The authority stays enabled, so only the name has to be set.
        let planner = AuthorityPlanner::new(make_current_authorities(), [make_authority(0, "Bob", true)]).unwrap();
        let expected = Authority { common_name: Some("Bob".into()), ..make_uid_only_authority(0) };
        assert_that!(planner.plan(), elements_are![eq(&make_step(expected))]);
    }

    // This is not a functional requirement, updates will work in any order.
    // However, it keeps errors and telemetry traces deterministic, which helps
    // with debugging.
    #[test]
    fn planner_plan_ordered_by_uid() {
        let desired = [
            make_authority(2, "Carol", true),
            make_authority(1, "Bob", true),
        ];
        let planner = AuthorityPlanner::new(make_current_authorities(), desired).unwrap();
        assert_that!(
            planner.plan(),
            elements_are![
                eq(&make_step(make_authority(1, "Bob", true))),
                eq(&make_step(make_authority(2, "Carol", true)))
            ]
        );
    }

    //--------------------------------------------------------------------------
    // is_matching_current
    //--------------------------------------------------------------------------
    #[rstest]
    #[case::equal(
        Authority{ common_name: None, enabled: Some(true), ..Default::default() },
        Authority{ common_name: None, enabled: Some(true), ..Default::default() },
        true,
    )]
    #[case::unspecified(
        Authority{ common_name: None, enabled: Some(true), ..Default::default() },
        Authority{ common_name: Some("Alice".into()), enabled: Some(true), ..Default::default() },
        true,
    )]
    #[case::overspecified(
        Authority{ common_name: Some("Alice".into()), enabled: Some(true), ..Default::default() },
        Authority{ common_name: None, enabled: Some(true), ..Default::default() },
        false,
    )]
    #[case::mismatching(
        Authority{ common_name: None, enabled: Some(true), ..Default::default() },
        Authority{ common_name: None, enabled: Some(false), ..Default::default() },
        false,
    )]
    fn is_matching_current_(#[case] expected: Authority, #[case] current: Authority, #[case] result: bool) {
        assert_eq!(expected.is_matching_current(&current), result);
    }
}
