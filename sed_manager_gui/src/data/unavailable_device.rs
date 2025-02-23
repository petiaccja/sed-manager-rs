use crate::ui::UnavailableDevice;

impl UnavailableDevice {
    pub fn new(path: String, error_message: String) -> Self {
        Self { error_message: error_message.into(), path: path.into() }
    }

    pub fn empty() -> Self {
        Self::new(String::new(), String::new())
    }
}
