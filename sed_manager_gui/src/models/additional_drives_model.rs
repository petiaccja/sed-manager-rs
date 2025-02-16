use slint::{ModelRc, VecModel};

use crate::generated::{AdditionalDrivesModel, ContentStatus};

impl AdditionalDrivesModel {
    pub fn empty() -> Self {
        Self {
            drives: ModelRc::new(VecModel::from(vec![])),
            error: String::new().into(),
            status: ContentStatus::Loading,
        }
    }

    pub fn new(drives: Vec<(String, String)>, error: String, status: ContentStatus) -> Self {
        Self {
            drives: ModelRc::new(VecModel::from(
                drives.into_iter().map(|(path, error)| (error.into(), path.into())).collect::<Vec<_>>(),
            )),
            error: error.into(),
            status,
        }
    }
}
