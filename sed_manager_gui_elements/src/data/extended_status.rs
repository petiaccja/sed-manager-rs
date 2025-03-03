use crate::{ExtendedStatus, Status};

impl ExtendedStatus {
    pub fn new(status: Status, message: String) -> Self {
        Self { status, message: message.into() }
    }

    pub fn loading() -> Self {
        Self::new(Status::Loading, String::new())
    }

    pub fn error(message: String) -> Self {
        Self::new(Status::Error, message)
    }

    pub fn success() -> Self {
        Self::new(Status::Success, String::new())
    }
}
