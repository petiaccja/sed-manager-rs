#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {}

impl core::error::Error for Error {}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown Linux platform error")
    }
}
