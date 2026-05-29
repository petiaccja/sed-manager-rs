//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_device::Device;

use crate::display_ui::DisplayUi;
use sed_manager_gui_slint as ui;

impl DisplayUi for dyn Device {
    type Ui = ui::Identity;

    fn display_ui(&self) -> Self::Ui {
        ui::Identity {
            name: self.model_number().into(),
            serial: self.serial_number().into(),
            path: self.path().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default().into(),
            firmware: self.firmware_revision().into(),
            interface: self.interface().to_string().into(),
            security_commands: self.is_security_supported(),
            is_removable: self.is_removable(),
        }
    }
}
