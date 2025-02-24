use crate::ui::{ActionResult, ContentStatus};

impl ActionResult {
    pub fn new(status: ContentStatus, error_message: String) -> Self {
        Self { status, error_message: error_message.into() }
    }

    pub fn loading() -> Self {
        Self::new(ContentStatus::Loading, String::new())
    }

    pub fn error(error_message: String) -> Self {
        Self::new(ContentStatus::Error, error_message)
    }

    pub fn success() -> Self {
        Self::new(ContentStatus::Success, String::new())
    }
}
