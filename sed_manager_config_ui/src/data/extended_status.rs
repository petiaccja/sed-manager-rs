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

    pub fn from_result<T, E: core::error::Error>(result: Result<T, E>) -> Self {
        match result {
            Ok(_) => Self::success(),
            Err(error) => Self::error(error.to_string()),
        }
    }
}

// This help us use ExtendedStatus with the ? operator.
impl<Error> From<Error> for ExtendedStatus
where
    Error: core::error::Error,
{
    fn from(value: Error) -> Self {
        Self::error(value.to_string())
    }
}
