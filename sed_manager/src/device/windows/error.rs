use super::super::shared::string::FromNullTerminated;
use crate::device;

use core::fmt::Display;
use core::ptr::null_mut;
use winapi::{
    shared::{
        minwindef::DWORD,
        winerror::{
            ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND, ERROR_INVALID_DATA, ERROR_INVALID_PARAMETER,
            ERROR_NOT_SUPPORTED, ERROR_PATH_NOT_FOUND, E_ACCESSDENIED, E_INVALIDARG, FAILED,
        },
    },
    um::{
        errhandlingapi::GetLastError,
        winbase::{FormatMessageW, FORMAT_MESSAGE_FROM_SYSTEM, FORMAT_MESSAGE_IGNORE_INSERTS},
        winnt::{HRESULT, LANG_NEUTRAL, MAKELANGID, SUBLANG_DEFAULT},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Win32(DWORD),
    COM(HRESULT),
}

impl core::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Win32(code) => match format_win32_error(*code) {
                Some(text) => write!(f, "{text}"),
                None => f.write_fmt(format_args!("Windows API error, code {code}")),
            },
            Error::COM(hr) => f.write_fmt(format_args!("Windows COM error, code {hr:10x}")),
        }
    }
}

impl From<Error> for device::Error {
    fn from(value: Error) -> device::Error {
        let common_error = match value {
            Error::Win32(code) => match code {
                ERROR_FILE_NOT_FOUND => Some(device::Error::DeviceNotFound),
                ERROR_PATH_NOT_FOUND => Some(device::Error::DeviceNotFound),
                ERROR_ACCESS_DENIED => Some(device::Error::PermissionDenied),
                ERROR_INVALID_DATA => Some(device::Error::InvalidArgument),
                ERROR_NOT_SUPPORTED => Some(device::Error::NotSupported),
                ERROR_INVALID_PARAMETER => Some(device::Error::InvalidArgument),
                _ => None,
            },
            Error::COM(hr) => match hr {
                E_ACCESSDENIED => Some(device::Error::PermissionDenied),
                E_INVALIDARG => Some(device::Error::InvalidArgument),
                _ => None,
            },
        };
        match common_error {
            Some(error) => error,
            None => device::Error::PlatformError(value),
        }
    }
}

pub fn get_last_error() -> Result<(), Error> {
    match unsafe { GetLastError() } {
        0 => Ok(()),
        code => Err(Error::Win32(code)),
    }
}

pub fn check_hresult(hr: HRESULT) -> Result<(), Error> {
    if FAILED(hr) {
        Err(Error::COM(hr))
    } else {
        Ok(())
    }
}

fn format_win32_error(code: DWORD) -> Option<String> {
    let mut buffer = vec![0u16; 4096];
    let num_written = unsafe {
        FormatMessageW(
            FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS,
            null_mut(),
            code,
            MAKELANGID(LANG_NEUTRAL, SUBLANG_DEFAULT) as u32,
            buffer.as_mut_ptr(),
            buffer.len() as u32,
            null_mut(),
        )
    };
    if num_written == 0 {
        return None;
    }
    match String::from_null_terminated_utf16(buffer.as_mut_ptr()) {
        Some(text) => Some(text),
        None => None,
    }
}
