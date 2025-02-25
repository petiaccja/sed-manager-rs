use crate::device::windows::error::{check_hresult, Error};
use core::ptr::null_mut;
use std::cell::OnceCell;
use std::sync::OnceLock;

use winapi::{
    shared::rpcdce::{RPC_C_AUTHN_LEVEL_DEFAULT, RPC_C_IMP_LEVEL_IMPERSONATE},
    um::{
        combaseapi::{CoInitializeEx, CoInitializeSecurity, CoUninitialize, COINITBASE_MULTITHREADED},
        objidlbase::EOAC_NONE,
    },
};

static COM_SECURITY: ComSecurity = ComSecurity { is_initialized: OnceLock::new() };
thread_local! {
pub static COM_INTERFACE: ComInterface = ComInterface{ is_initialized: OnceCell::new() };
}

struct ComSecurity {
    is_initialized: OnceLock<()>,
}

pub struct ComInterface {
    is_initialized: OnceCell<()>,
}

impl ComSecurity {
    pub fn init(&self) -> Result<(), Error> {
        // This does not actually return the error.
        // But guess what, I don't give a fuck because this COM
        // initialize garbage is the second biggest pile of garbage
        // that I've seen after OpenGL.
        // Worst case drive detection won't work...
        self.is_initialized.get_or_init(|| -> () {
            let _ = co_initialize_security();
        });
        Ok(())
    }
}

impl ComInterface {
    pub fn init(&self) -> Result<(), Error> {
        match self.is_initialized.get() {
            Some(_) => Ok(()),
            None => {
                co_initialize_ex()?;
                COM_SECURITY.init()?;
                Ok(*self.is_initialized.get_or_init(|| -> () { () }))
            }
        }
    }
}

impl Drop for ComInterface {
    fn drop(&mut self) {
        co_uninitialize();
    }
}

fn co_initialize_ex() -> Result<(), Error> {
    unsafe { check_hresult(CoInitializeEx(null_mut(), COINITBASE_MULTITHREADED)) }
}

fn co_uninitialize() {
    unsafe {
        CoUninitialize();
    }
}

fn co_initialize_security() -> Result<(), Error> {
    unsafe {
        check_hresult(CoInitializeSecurity(
            null_mut(),
            -1,
            null_mut(),
            null_mut(),
            RPC_C_AUTHN_LEVEL_DEFAULT,
            RPC_C_IMP_LEVEL_IMPERSONATE,
            null_mut(),
            EOAC_NONE,
            null_mut(),
        ))
    }
}
