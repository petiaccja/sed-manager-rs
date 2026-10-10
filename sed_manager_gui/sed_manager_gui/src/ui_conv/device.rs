//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_device::StorageDevice;

use crate::ui_conv::FromBackend;
use sed_manager_gui_slint as ui;

impl FromBackend<&dyn StorageDevice> for ui::Identity {
    fn from_backend(value: &dyn StorageDevice) -> Self {
        ui::Identity {
            status: ui::Status::default(),
            name: value.model_number().into(),
            serial: value.serial_number().into(),
            path: value.path().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default().into(),
            firmware: value.firmware_revision().into(),
            interface: value.interface().to_string().into(),
            security_commands: value.is_security_supported(),
            is_removable: value.is_removable(),
        }
    }
}
