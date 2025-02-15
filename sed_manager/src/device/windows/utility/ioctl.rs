use std::ffi::c_void;
use std::ptr::null_mut;

use winapi::{
    shared::{
        minwindef::{DWORD, FALSE},
        ntdef::{BOOLEAN, HANDLE, UCHAR, ULONG},
    },
    um::ioapiset::DeviceIoControl,
};

use crate::device::windows::error::{get_last_error, Error};

pub fn ioctl_in_out(device: HANDLE, ioctl: DWORD, buffer: &mut [u8]) -> Result<u32, Error> {
    let mut bytes_returned: u32 = 0;
    let result = unsafe {
        DeviceIoControl(
            device,
            ioctl,
            buffer.as_mut_ptr() as *mut c_void,
            buffer.len() as u32,
            buffer.as_mut_ptr() as *mut c_void,
            buffer.len() as u32,
            &mut bytes_returned as *mut u32,
            null_mut(),
        )
    };

    if result == FALSE {
        get_last_error()?;
    };
    Ok(bytes_returned)
}

#[repr(C)]
#[allow(nonstandard_style)]
#[allow(unused)]
pub struct STORAGE_DEVICE_DESCRIPTOR {
    pub Version: ULONG,
    pub Size: ULONG,
    pub DeviceType: UCHAR,
    pub DeviceTypeModifier: UCHAR,
    pub RemovableMedia: BOOLEAN,
    pub CommandQueueing: BOOLEAN,
    pub VendorIdOffset: ULONG,
    pub ProductIdOffset: ULONG,
    pub ProductRevisionOffset: ULONG,
    pub SerialNumberOffset: ULONG,
    pub BusType: STORAGE_BUS_TYPE,
    pub RawPropertiesLength: ULONG,
    pub RawDeviceProperties: [UCHAR; 1],
}

#[repr(u32)]
#[allow(nonstandard_style)]
#[allow(unused)]
#[derive(Debug)]
pub enum STORAGE_PROTOCOL_TYPE {
    ProtocolTypeUnknown = 0x00,
    ProtocolTypeScsi,
    ProtocolTypeAta,
    ProtocolTypeNvme,
    ProtocolTypeSd,
    ProtocolTypeUfs,
    ProtocolTypeProprietary = 0x7E,
    ProtocolTypeMaxReserved = 0x7F,
}

#[repr(C)]
#[allow(nonstandard_style)]
#[allow(unused)]
pub struct STORAGE_PROTOCOL_SPECIFIC_DATA {
    pub ProtocolType: STORAGE_PROTOCOL_TYPE,
    pub DataType: DWORD,
    pub ProtocolDataRequestValue: DWORD,
    pub ProtocolDataRequestSubValue: DWORD,
    pub ProtocolDataOffset: DWORD,
    pub ProtocolDataLength: DWORD,
    pub FixedProtocolReturnData: DWORD,
    pub ProtocolDataRequestSubValue2: DWORD,
    pub ProtocolDataRequestSubValue3: DWORD,
    pub ProtocolDataRequestSubValue4: DWORD,
}

#[repr(C)]
#[allow(nonstandard_style)]
#[allow(unused)]
#[derive(Debug)]
pub enum STORAGE_BUS_TYPE {
    BusTypeUnknown,
    BusTypeScsi,
    BusTypeAtapi,
    BusTypeAta,
    BusType1394,
    BusTypeSsa,
    BusTypeFibre,
    BusTypeUsb,
    BusTypeRAID,
    BusTypeiScsi,
    BusTypeSas,
    BusTypeSata,
    BusTypeSd,
    BusTypeMmc,
    BusTypeVirtual,
    BusTypeFileBackedVirtual,
    BusTypeSpaces,
    BusTypeNvme,
    BusTypeSCM,
    BusTypeUfs,
    BusTypeNvmeof,
    BusTypeMax,
    BusTypeMaxReserved,
}
