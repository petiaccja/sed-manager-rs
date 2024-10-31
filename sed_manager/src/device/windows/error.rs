use crate::device;

use super::string::null_terminated_to_string;
use std::{fmt::Display, ptr::null_mut};
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

#[derive(Debug, Clone, Copy)]
pub enum Error {
    Win32(DWORD),
    COM(HRESULT),
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Win32(code) => match format_win32_error(*code) {
                Some(text) => f.write_fmt(format_args!("{}", text)),
                None => f.write_fmt(format_args!("Win32 error: {}", code)),
            },
            Error::COM(hr) => f.write_fmt(format_args!("COM error: {}", hr)),
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
            None => device::Error::Source(value),
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
    match null_terminated_to_string(buffer.as_mut_ptr()) {
        Ok(text) => Some(text),
        Err(_) => None,
    }
}
