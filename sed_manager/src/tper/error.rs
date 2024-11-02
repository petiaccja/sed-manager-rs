use crate::device::Error as DeviceError;
use crate::serialization::Error as SerializeError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("error decoding/encoding data: {0}")]
    InvalidFormat(SerializeError),
    #[error("error communicating with device: {0}")]
    InvalidRequest(DeviceError),
}

impl From<SerializeError> for Error {
    fn from(value: SerializeError) -> Self {
        Error::InvalidFormat(value)
    }
}

impl From<DeviceError> for Error {
    fn from(value: DeviceError) -> Self {
        Error::InvalidRequest(value)
    }
}
