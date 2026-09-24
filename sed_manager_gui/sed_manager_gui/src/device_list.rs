//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::path::PathBuf;
use std::{collections::HashMap, sync::Arc};

use async_lock::{Mutex, RwLock};
use sed_manager::Device;
use sed_manager_gui_slint as ui;

use crate::associative_model::AssociativeModel;
use crate::session::Session;

#[derive(Debug, Default)]
pub struct DeviceList {
    pub ui: AssociativeModel<PathBuf, ui::Device>,
    pub backend: HashMap<PathBuf, Arc<RwLock<DeviceEntry>>>,
}

#[derive(Debug, Default)]
pub struct DeviceEntry {
    pub device: Option<Device>,
    pub session: Arc<Mutex<Session>>,
}

impl DeviceEntry {
    pub async fn close(&mut self) {
        let device = self.device.take();
        let mut session = self.session.lock().await;

        // Close the session, and issue a stack reset if it fails.
        // The stack reset is needed because otherwise hanging sessions
        // could block closing the Tper.
        if let Err(_) = session.close().await
            && let Some(tper) = device.as_ref()
        {
            let _ = tper.stack_reset().await;
        }

        // Close the TPer. This makes sure all session had the chance to terminate.
        // This call could hang the application if someone still holds a session or
        // if there is a bug in the protocol logic.
        if let Some(device) = device {
            let _ = device.close().await;
        }
    }
}
