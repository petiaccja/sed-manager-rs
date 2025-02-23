use crate::ui::NameValuePair;

impl NameValuePair {
    pub fn new(name: String, value: String) -> Self {
        Self { name: name.into(), value: value.into() }
    }
}
