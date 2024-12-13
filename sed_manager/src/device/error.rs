#[cfg(target_os = "windows")]
use super::windows::Error as PlatformError;

#[cfg(target_os = "linux")]
use super::linux::Error as PlatformError;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Error {
    DataTooLong,
    BufferTooShort,
    DeviceNotFound,
    InvalidArgument,
    InvalidProtocolOrComID,
    NotImplemented,
    NotSupported,
    PermissionDenied,
    Source(PlatformError),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::DataTooLong => f.write_fmt(format_args!("provided data is too long")),
            Error::BufferTooShort => f.write_fmt(format_args!("provided buffer is too short")),
            Error::DeviceNotFound => f.write_fmt(format_args!("could not find the specified device")),
            Error::InvalidArgument => f.write_fmt(format_args!("invalid argument")),
            Error::InvalidProtocolOrComID => f.write_fmt(format_args!("the protocol / com ID pair is invalid")),
            Error::NotImplemented => f.write_fmt(format_args!("not implemented")),
            Error::NotSupported => f.write_fmt(format_args!("not supported")),
            Error::PermissionDenied => f.write_fmt(format_args!("permission denied")),
            Error::Source(err) => f.write_fmt(format_args!("{}", err)),
        }
    }
}
