use sed_manager::messaging::com_id::ComIdState;
use slint::{ModelRc, VecModel};

use crate::{ConfigureState, ExtendedStatus, LockingRange, LockingRangeState, TroubleshootState};

impl ConfigureState {
    pub fn new(extended_status: ExtendedStatus, locking_ranges: LockingRangeState) -> Self {
        Self { extended_status, locking_ranges }
    }

    pub fn empty() -> Self {
        Self::new(ExtendedStatus::error("not implemented".into()), LockingRangeState::empty())
    }
}

impl TroubleshootState {
    pub fn new(com_id: u16, com_id_ext: u16, com_id_status: ComIdState, extended_status: ExtendedStatus) -> Self {
        let status_str = match com_id_status {
            ComIdState::Invalid => "invalid",
            ComIdState::Inactive => "invactive",
            ComIdState::Issued => "issued",
            ComIdState::Associated => "associated",
        };
        Self {
            com_id: com_id as i32,
            com_id_ext: com_id_ext as i32,
            com_id_status: status_str.into(),
            extended_status,
        }
    }

    pub fn empty() -> Self {
        Self::new(0, 0, ComIdState::Invalid, ExtendedStatus::error("not implemented".into()))
    }
}

impl LockingRangeState {
    pub fn new(names: Vec<String>, properties: Vec<LockingRange>, statuses: Vec<ExtendedStatus>) -> Self {
        let names: Vec<_> = names.into_iter().map(|x| x.into()).collect();
        Self {
            names: ModelRc::new(VecModel::from(names)),
            properties: ModelRc::new(VecModel::from(properties)),
            statuses: ModelRc::new(VecModel::from(statuses)),
        }
    }

    pub fn empty() -> Self {
        Self::new(Vec::new(), Vec::new(), Vec::new())
    }
}
