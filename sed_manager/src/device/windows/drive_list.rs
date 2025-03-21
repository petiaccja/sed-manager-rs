//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::ptr::null_mut;
use core::{ffi::c_void, iter::from_fn, mem::zeroed, ops::Deref};
use winapi::{
    shared::{
        guiddef::GUID,
        rpcdce::{RPC_C_AUTHN_LEVEL_CALL, RPC_C_AUTHN_WINNT, RPC_C_AUTHZ_NONE, RPC_C_IMP_LEVEL_IMPERSONATE},
        winerror::E_FAIL,
        wtypesbase::CLSCTX_INPROC_SERVER,
    },
    um::{
        combaseapi::{CoCreateInstance, CoSetProxyBlanket},
        oaidl::VARIANT,
        objidl::EOAC_NONE,
        oleauto::{VariantClear, VariantInit},
        unknwnbase::IUnknown,
        wbemcli::{
            CLSID_WbemLocator, IEnumWbemClassObject, IID_IWbemLocator, IWbemClassObject, IWbemLocator, IWbemServices,
            WBEM_FLAG_FORWARD_ONLY, WBEM_FLAG_RETURN_IMMEDIATELY, WBEM_INFINITE, WBEM_S_FALSE,
        },
    },
};

use crate::device::shared::string::{FromNullTerminated, ToNullTerminated};
use crate::device::Error as DeviceError;

use super::error::{check_hresult, Error as WindowsError};
use super::utility::{com_interface::COM_INTERFACE, com_ptr::ComPtr};

fn co_create_instance<T: Deref<Target = IUnknown>>(clsid: &GUID, riid: &GUID) -> Result<ComPtr<T>, WindowsError> {
    let mut ptr = ComPtr::<T>::null();
    let result = unsafe {
        check_hresult(CoCreateInstance(
            clsid as *const GUID,
            null_mut(),
            CLSCTX_INPROC_SERVER,
            riid as *const GUID,
            ptr.as_mut() as *mut *mut T as *mut *mut c_void,
        ))
    };
    match result {
        Ok(_) => Ok(ptr),
        Err(err) => Err(err),
    }
}

fn get_wbem_services(
    wbem_locator: *mut IWbemLocator,
    network_resource: &str,
) -> Result<ComPtr<IWbemServices>, WindowsError> {
    let mut network_resource_utf16: Vec<_> = network_resource.to_null_terminated_utf16();
    let mut ptr = ComPtr::<IWbemServices>::null();
    let result = unsafe {
        check_hresult((*wbem_locator).ConnectServer(
            network_resource_utf16.as_mut_ptr(),
            null_mut(),
            null_mut(),
            null_mut(),
            0,
            null_mut(),
            null_mut(),
            ptr.as_mut() as *mut *mut IWbemServices,
        ))
    };
    match result {
        Ok(_) => Ok(ptr),
        Err(err) => Err(err),
    }
}

fn co_set_proxy_blanket(wbem_services: *mut IWbemServices) -> Result<(), WindowsError> {
    unsafe {
        check_hresult(CoSetProxyBlanket(
            wbem_services as *mut IUnknown,
            RPC_C_AUTHN_WINNT,
            RPC_C_AUTHZ_NONE,
            null_mut(),
            RPC_C_AUTHN_LEVEL_CALL,
            RPC_C_IMP_LEVEL_IMPERSONATE,
            null_mut(),
            EOAC_NONE,
        ))
    }
}

fn exec_query(wbem_services: *mut IWbemServices, query: &str) -> Result<ComPtr<IEnumWbemClassObject>, WindowsError> {
    let mut language_utf16: Vec<_> = "WQL".to_null_terminated_utf16();
    let mut query_utf16: Vec<_> = query.to_null_terminated_utf16();
    let mut ptr = ComPtr::<IEnumWbemClassObject>::null();
    let result = unsafe {
        check_hresult((*wbem_services).ExecQuery(
            language_utf16.as_mut_ptr(),
            query_utf16.as_mut_ptr(),
            (WBEM_FLAG_FORWARD_ONLY | WBEM_FLAG_RETURN_IMMEDIATELY) as i32,
            null_mut(),
            ptr.as_mut(),
        ))
    };
    match result {
        Ok(_) => Ok(ptr),
        Err(err) => Err(err),
    }
}

fn map_enumerator(
    enumerator: *mut IEnumWbemClassObject,
) -> core::iter::FromFn<impl FnMut() -> Option<ComPtr<IWbemClassObject>>> {
    from_fn(move || -> Option<ComPtr<IWbemClassObject>> {
        let mut returned: u32 = 0;
        let mut ptr = ComPtr::<IWbemClassObject>::null();
        let hr = unsafe { (*enumerator).Next(WBEM_INFINITE as i32, 1, ptr.as_mut(), &mut returned as *mut u32) };
        if hr == WBEM_S_FALSE as i32 {
            return None;
        }
        match check_hresult(hr) {
            Ok(_) => Some(ptr),
            Err(_) => None,
        }
    })
}

fn get_drive_path(object: *mut IWbemClassObject) -> Result<String, WindowsError> {
    let mut path_utf16: Vec<_> = "Path".to_null_terminated_utf16();
    let path = unsafe {
        // Do not return within this unsafe block.
        let mut property: VARIANT = zeroed();
        VariantInit(&mut property as *mut VARIANT);
        let result = check_hresult((*object).Get(
            path_utf16.as_mut_ptr(),
            0,
            &mut property as *mut VARIANT,
            null_mut(),
            null_mut(),
        ));
        let path = match result {
            Ok(_) => {
                let s = property.n1.n2().n3.bstrVal();
                match String::from_null_terminated_utf16(*s) {
                    Some(path) => Ok(path),
                    None => Err(WindowsError::COM(E_FAIL)),
                }
            }
            Err(err) => Err(err),
        };
        // This must be called to clean up resources.
        VariantClear(&mut property as *mut VARIANT);
        path
    }?;
    Ok(path)
}

pub fn list_physical_drives() -> Result<Vec<String>, DeviceError> {
    COM_INTERFACE.with(|com_interface| -> Result<(), WindowsError> { com_interface.init() })?;

    let wbem_locator = co_create_instance::<IWbemLocator>(&CLSID_WbemLocator, &IID_IWbemLocator)?;
    let wbem_services = get_wbem_services(wbem_locator.get(), r"ROOT\Microsoft\Windows\Storage")?;
    co_set_proxy_blanket(wbem_services.get())?;
    let enumerator = exec_query(wbem_services.get(), r"SELECT * FROM MSFT_Disk")?;
    Ok(map_enumerator(enumerator.get())
        .map(|object| -> Result<String, WindowsError> { get_drive_path(object.get()) })
        .filter_map(|result| -> Option<String> { result.ok() })
        .collect::<Vec<_>>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use skip_test::{may_skip, skip};

    #[test]
    #[may_skip]
    fn test_get_physical_drives() -> Result<(), DeviceError> {
        // There must be at least one physical drive, so this test should pass.
        match list_physical_drives() {
            Ok(physical_drives) => {
                assert!(!physical_drives.is_empty());
                Ok(())
            }
            Err(DeviceError::PermissionDenied) => skip!(),
            Err(err) => Err(err),
        }
    }
}
