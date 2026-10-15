//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::collections::BTreeMap;

use sed_spec::objects::{Authority, AuthorityRef};

use crate::set_plan::{
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
