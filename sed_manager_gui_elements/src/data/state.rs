use slint::{ModelRc, ToSharedString, VecModel};

use crate::{ExtendedStatus, LockingRange, PermissionList, PermissionMatrix, RangeList, User, UserList};

impl RangeList {
    pub fn new(names: Vec<String>, values: Vec<LockingRange>, statuses: Vec<ExtendedStatus>) -> Self {
        let names: Vec<_> = names.into_iter().map(|x| x.to_shared_string()).collect();
        Self {
            names: ModelRc::new(VecModel::from(names)),
            values: ModelRc::new(VecModel::from(values)),
            statuses: ModelRc::new(VecModel::from(statuses)),
        }
    }

    pub fn empty() -> Self {
        Self::new(vec![], vec![], vec![])
    }
}

impl UserList {
    pub fn new(names: Vec<String>, values: Vec<User>, statuses: Vec<ExtendedStatus>) -> Self {
        let names: Vec<_> = names.into_iter().map(|x| x.to_shared_string()).collect();
        Self {
            names: ModelRc::new(VecModel::from(names)),
            values: ModelRc::new(VecModel::from(values)),
            statuses: ModelRc::new(VecModel::from(statuses)),
        }
    }

    pub fn empty() -> Self {
        Self::new(vec![], vec![], vec![])
    }
}

impl PermissionMatrix {
    pub fn new(users: Vec<String>, ranges: Vec<String>, permission_lists: Vec<PermissionList>) -> Self {
        let users: Vec<slint::SharedString> = users.into_iter().map(|x| x.into()).collect();
        let ranges: Vec<slint::SharedString> = ranges.into_iter().map(|x| x.into()).collect();
        Self {
            permission_lists: ModelRc::new(VecModel::from(permission_lists)),
            range_names: ModelRc::new(VecModel::from(ranges)),
            user_names: ModelRc::new(VecModel::from(users)),
        }
    }

    pub fn empty() -> Self {
        Self::new(vec![], vec![], vec![])
    }
}

impl PermissionList {
    pub fn new(
        unshadow_mbr: bool,
        unshadow_mbr_status: ExtendedStatus,
        read_unlock: Vec<bool>,
        write_unlock: Vec<bool>,
        range_statuses: Vec<ExtendedStatus>,
    ) -> Self {
        Self {
            range_statuses: ModelRc::new(VecModel::from(range_statuses.clone())),
            read_unlock: ModelRc::new(VecModel::from(read_unlock.clone())),
            unshadow_mbr,
            unshadow_mbr_status,
            write_unlock: ModelRc::new(VecModel::from(write_unlock)),
        }
    }

    pub fn blank(num_ranges: usize) -> Self {
        let statuses = core::iter::repeat_n(ExtendedStatus::loading(), num_ranges);
        let flags = core::iter::repeat_n(false, num_ranges);
        Self::new(false, ExtendedStatus::loading(), flags.clone().collect(), flags.collect(), statuses.collect())
    }
}
