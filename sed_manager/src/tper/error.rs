use crate::device;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    InvalidResponse,
    DeviceError(device::Error),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidResponse => f.write_fmt(format_args!("invalid response")),
            Error::DeviceError(device_err) => f.write_fmt(format_args!("device error: {}", device_err)),
        }
    }
}

impl From<device::Error> for Error {
    fn from(value: device::Error) -> Self {
        Error::DeviceError(value)
    }
}
