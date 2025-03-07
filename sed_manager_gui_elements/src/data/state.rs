use slint::{ModelRc, ToSharedString, VecModel};

use crate::{ExtendedStatus, LockingRange, RangeList, User, UserList};

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
