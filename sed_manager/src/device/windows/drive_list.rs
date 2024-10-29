use core::ptr::null_mut;
use std::{iter::from_fn, mem::zeroed, ops::Deref, os::raw::c_void};
use winapi::{
    shared::{
        guiddef::GUID,
        rpcdce::{
            RPC_C_AUTHN_LEVEL_CALL, RPC_C_AUTHN_LEVEL_DEFAULT, RPC_C_AUTHN_WINNT, RPC_C_AUTHZ_NONE,
            RPC_C_IMP_LEVEL_IMPERSONATE,
        },
        winerror::S_OK,
        wtypesbase::CLSCTX_INPROC_SERVER,
    },
    um::{
        combaseapi::{
            CoCreateInstance, CoInitializeEx, CoInitializeSecurity, CoSetProxyBlanket, CoUninitialize,
            COINITBASE_MULTITHREADED,
        },
        oaidl::VARIANT,
        objidl::EOAC_NONE,
        oleauto::{VariantClear, VariantInit},
        unknwnbase::IUnknown,
        wbemcli::{
            CLSID_WbemLocator, IEnumWbemClassObject, IID_IWbemLocator, IWbemClassObject, IWbemLocator, IWbemServices,
            WBEM_FLAG_FORWARD_ONLY, WBEM_FLAG_RETURN_IMMEDIATELY, WBEM_INFINITE,
        },
    },
};

use crate::device::device::{DeviceError, Interface};

use super::{
    com_ptr::ComPtr,
    error::{get_last_error, result_to_error, Error},
    string::{null_terminated_to_string, string_to_wchars},
};

fn co_initialize_ex() -> Result<(), Error> {
    unsafe { result_to_error(CoInitializeEx(null_mut(), COINITBASE_MULTITHREADED)) }
}

fn co_uninitialize() {
    unsafe {
        CoUninitialize();
    }
}

fn co_initialize_security() -> Result<(), Error> {
    unsafe {
        let result = CoInitializeSecurity(
            null_mut(),
            -1,
            null_mut(),
            null_mut(),
            RPC_C_AUTHN_LEVEL_DEFAULT,
            RPC_C_IMP_LEVEL_IMPERSONATE,
            null_mut(),
            EOAC_NONE,
            null_mut(),
        );
        result_to_error(result)
    }
}

fn co_create_instance<T: Deref<Target = IUnknown>>(clsid: &GUID, riid: &GUID) -> Result<ComPtr<T>, Error> {
    let mut ptr = ComPtr::<T>::null();
    let result = unsafe {
        CoCreateInstance(
            clsid as *const GUID,
            null_mut(),
            CLSCTX_INPROC_SERVER,
            riid as *const GUID,
            ptr.as_mut() as *mut *mut T as *mut *mut c_void,
        )
    };
    match result_to_error(result) {
        Ok(_) => Ok(ptr),
        Err(err) => Err(err),
    }
}

fn get_wbem_services(
    wbem_locator: *mut IWbemLocator,
    network_resource: &str,
) -> Result<ComPtr<IWbemServices>, Error> {
    let mut network_resource_utf16: Vec<_> = string_to_wchars(network_resource);
    let mut ptr = ComPtr::<IWbemServices>::null();
    let result = unsafe {
        (*wbem_locator).ConnectServer(
            network_resource_utf16.as_mut_ptr(),
            null_mut(),
            null_mut(),
            null_mut(),
            0,
            null_mut(),
            null_mut(),
            ptr.as_mut() as *mut *mut IWbemServices,
        )
    };
    match result_to_error(result) {
        Ok(_) => Ok(ptr),
        Err(err) => Err(err),
    }
}

fn co_set_proxy_blanket(wbem_services: *mut IWbemServices) -> Result<(), Error> {
    unsafe {
        let result = CoSetProxyBlanket(
            wbem_services as *mut IUnknown,
            RPC_C_AUTHN_WINNT,
            RPC_C_AUTHZ_NONE,
            null_mut(),
            RPC_C_AUTHN_LEVEL_CALL,
            RPC_C_IMP_LEVEL_IMPERSONATE,
            null_mut(),
            EOAC_NONE,
        );
        result_to_error(result)
    }
}

fn exec_query(wbem_services: *mut IWbemServices, query: &str) -> Result<ComPtr<IEnumWbemClassObject>, Error> {
    let mut language_utf16: Vec<_> = string_to_wchars("WQL");
    let mut query_utf16: Vec<_> = string_to_wchars(query);
    let mut ptr = ComPtr::<IEnumWbemClassObject>::null();
    unsafe {
        let result = (*wbem_services).ExecQuery(
            language_utf16.as_mut_ptr(),
            query_utf16.as_mut_ptr(),
            (WBEM_FLAG_FORWARD_ONLY | WBEM_FLAG_RETURN_IMMEDIATELY) as i32,
            null_mut(),
            ptr.as_mut(),
        );
        match result_to_error(result) {
            Ok(_) => Ok(ptr),
            Err(err) => Err(err),
        }
    }
}

fn map_enumerator(
    enumerator: *mut IEnumWbemClassObject,
) -> std::iter::FromFn<impl FnMut() -> Option<ComPtr<IWbemClassObject>>> {
    from_fn(move || -> Option<ComPtr<IWbemClassObject>> {
        let mut returned: u32 = 0;
        let mut ptr = ComPtr::<IWbemClassObject>::null();
        let result = unsafe { (*enumerator).Next(WBEM_INFINITE as i32, 1, ptr.as_mut(), &mut returned as *mut u32) };
        match result_to_error(result) {
            Ok(_) => Some(ptr),
            Err(_) => None,
        }
    })
}

fn get_drive_properties(object: *mut IWbemClassObject) -> Result<(String, Interface), Error> {
    let mut bus_type_utf16: Vec<_> = string_to_wchars("BusType");
    let mut path_utf16: Vec<_> = string_to_wchars("Path");
    let path = unsafe {
        // Do not return within this unsafe block.
        let mut property: VARIANT = zeroed();
        VariantInit(&mut property as *mut VARIANT);
        let result = (*object).Get(path_utf16.as_mut_ptr(), 0, &mut property as *mut VARIANT, null_mut(), null_mut());
        let path = if result == S_OK {
            let s = property.n1.n2().n3.bstrVal();
            match null_terminated_to_string(*s) {
                Ok(path) => Ok(path),
                Err(_) => Err(Error { error_code: 0x80004005 }),
            }
        } else {
            Err(get_last_error())
        };
        // This must be called to clean up resources.
        VariantClear(&mut property as *mut VARIANT);
        path
    }?;
    let bus_type = unsafe {
        // Do not return within this unsafe block.
        let mut property: VARIANT = zeroed();
        VariantInit(&mut property as *mut VARIANT);
        let result =
            (*object).Get(bus_type_utf16.as_mut_ptr(), 0, &mut property as *mut VARIANT, null_mut(), null_mut());
        let bus_type = if result == S_OK { Ok(*property.n1.n2().n3.uiVal()) } else { Err(get_last_error()) };
        // This must be called to clean up resources.
        VariantClear(&mut property as *mut VARIANT);
        bus_type
    }?;
    let interface = match bus_type {
        1 => Interface::SCSI,
        3 => Interface::ATA,
        11 => Interface::SATA,
        12 => Interface::SD,
        13 => Interface::MMC,
        17 => Interface::NVMe,
        _ => Interface::Other,
    };
    Ok((path, interface))
}

fn get_physical_drives_and_interfaces() -> Result<Vec<(String, Interface)>, DeviceError> {
    fn with_co_initialized() -> Result<Vec<(String, Interface)>, DeviceError> {
        co_initialize_security()?;
        let wbem_locator = co_create_instance::<IWbemLocator>(&CLSID_WbemLocator, &IID_IWbemLocator)?;
        let wbem_services = get_wbem_services(wbem_locator.get(), r"ROOT\Microsoft\Windows\Storage")?;
        co_set_proxy_blanket(wbem_services.get())?;
        let enumerator = exec_query(wbem_services.get(), r"SELECT * FROM MSFT_Disk")?;
        Ok(map_enumerator(enumerator.get())
            .map(|object| -> Result<(String, Interface), Error> { get_drive_properties(object.get()) })
            .filter_map(|result| -> Option<(String, Interface)> { result.ok() })
            .collect::<Vec<_>>())
    }

    let init_res = co_initialize_ex();
    let result = match init_res {
        Ok(_) => {
            let result = with_co_initialized();
            result
        }
        Err(err) => Err(err.into()),
    };
    co_uninitialize();
    result
}

pub fn get_physical_drives() -> Result<Vec<String>, DeviceError> {
    Ok(get_physical_drives_and_interfaces()?.into_iter().map(|d| d.0).collect())
}

pub fn get_physical_drive_interface(drive_path: &str) -> Result<Interface, DeviceError> {
    for drive in get_physical_drives_and_interfaces()? {
        if drive.0 == drive_path {
            return Ok(drive.1);
        }
    }
    Err(DeviceError::NotFound)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_physical_drives() {
        // There must be at least one physical drive.
        match get_physical_drives() {
            Ok(physical_drives) => {
                assert!(!physical_drives.is_empty());
            }
            Err(err) => panic!("failed to get list of drives: {}", err),
        };
    }

    #[test]
    fn test_get_physical_drive_interface() {
        if let Ok(physical_drives) = get_physical_drives() {
            if let Some(drive) = physical_drives.first() {
                match get_physical_drive_interface(drive) {
                    Ok(_) => (),
                    Err(err) => panic!("failed to get drive interface {}", err),
                };
            }
        };
    }
}
