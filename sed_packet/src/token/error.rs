use sorbit::error::Error as SorbitError;

pub trait MessageError {
    fn message(message: impl Into<String>) -> Self;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("Can not serialize the token: {}", .0)]
    CanNotSerialize(SorbitError),
    #[error("The payload ({len} bytes) does not fit into a large atom")]
    PayloadTooBig { len: usize },
    #[error("Can not convert `{from}` into `{to}`")]
    CanNotConvert { from: &'static str, to: &'static str },
    #[error("Expected an EndNamed token")]
    ExpectedEndNamed,
    #[error("Did not expect an EndNamed token at this point")]
    UnexpectedEndNamed,
    #[error("Did not expect an EndList token at this point")]
    UnexpectedEndList,
    #[error("{}", .0)]
    Custom(String),
}

impl From<SorbitError> for Error {
    fn from(value: SorbitError) -> Self {
        Self::CanNotSerialize(value)
    }
}

impl MessageError for Error {
    fn message(message: impl Into<String>) -> Self {
        Self::Custom(message.into())
    }
}
