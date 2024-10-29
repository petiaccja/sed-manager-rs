use super::string::null_terminated_to_string;
use crate::device::device::DeviceError;
use std::{fmt::Display, ptr::null_mut};
use winapi::{
    shared::winerror::S_OK,
    um::{
        errhandlingapi::GetLastError,
        winbase::{FormatMessageW, FORMAT_MESSAGE_FROM_SYSTEM, FORMAT_MESSAGE_IGNORE_INSERTS},
        winnt::{HRESULT, LANG_NEUTRAL, MAKELANGID, SUBLANG_DEFAULT},
    },
};

pub struct Error {
    pub error_code: u32,
}

impl From<Error> for DeviceError {
    fn from(value: Error) -> Self {
        DeviceError::OSError(value)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut buffer = vec![0u16; 4096];
        let count = unsafe {
            FormatMessageW(
                FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS,
                null_mut(),
                self.error_code,
                MAKELANGID(LANG_NEUTRAL, SUBLANG_DEFAULT) as u32,
                buffer.as_mut_ptr(),
                buffer.len() as u32,
                null_mut(),
            )
        };
        if count != 0 {
            match null_terminated_to_string(buffer.as_mut_ptr()) {
                Ok(s) => f.write_fmt(format_args!("{}", s)),
                Err(_) => f.write_fmt(format_args!("{}", self.error_code)),
            }
        } else {
            println!("problem formatting windows error code: {}", unsafe { GetLastError() });
            f.write_fmt(format_args!("{}", self.error_code))
        }
    }
}

pub fn get_last_error() -> Error {
    let ec = unsafe { GetLastError() };
    Error { error_code: ec }
}

pub fn result_to_error(result: HRESULT) -> Result<(), Error> {
    if result != S_OK {
        Err(Error { error_code: unsafe { std::mem::transmute(result) } })
    } else {
        Ok(())
    }
}
