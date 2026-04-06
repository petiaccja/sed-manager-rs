use sorbit::error::Error as SorbitError;

pub trait MessageError {
    fn message(message: impl Into<String>) -> Self;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("Serializing/deserializing the token failed: {}", .0)]
    SerializationFailed(SorbitError),
    #[error("The payload does not fit into the atom")]
    OversizedPayload,
    #[error("The token is valid, but it stores a value of a different type")]
    InvalidDataType,
    #[error("Expected an EndList token")]
    ExpectedStartNamed,
    #[error("Expected an EndNamed token")]
    ExpectedEndNamed,
    #[error("Did not expect an EndNamed token at this point")]
    UnexpectedEndNamed,
    #[error("Expected a StartList token")]
    ExpectedStartList,
    #[error("Did not expect an EndList token at this point")]
    UnexpectedEndList,
    #[error("{}", .0)]
    Custom(String),
}

impl From<SorbitError> for Error {
    fn from(value: SorbitError) -> Self {
        Self::SerializationFailed(value)
    }
}

impl MessageError for Error {
    fn message(message: impl Into<String>) -> Self {
        Self::Custom(message.into())
    }
}
