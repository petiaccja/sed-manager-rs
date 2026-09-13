//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::path::PathBuf;
use std::{collections::HashMap, sync::Arc};

use async_lock::{Mutex, RwLock};
use sed_manager::Spec;
use sed_manager_gui_slint as ui;
use sed_tper::Tper;

use crate::associative_model::AssociativeModel;
use crate::session::Session;

#[derive(Debug, Default)]
pub struct DeviceList {
    pub ui: AssociativeModel<PathBuf, ui::Device>,
    pub backend: HashMap<PathBuf, Arc<RwLock<Device>>>,
}

#[derive(Debug, Default)]
pub struct Device {
    pub interface: Option<Arc<dyn sed_device::Device>>,
    pub specification: Option<Spec>,
    pub tper: Option<Tper>,
    pub session: Arc<Mutex<Session>>,
}

impl Device {
    pub async fn close(&mut self) {
        let tper = self.tper.take();
        let mut session = self.session.lock().await;

        // Close the session, and issue a stack reset if it fails.
        // The stack reset is needed because otherwise hanging sessions
        // could block closing the Tper.
        if let Err(_) = session.close().await {
            if let Some(tper) = tper.as_ref() {
                let _ = tper.stack_reset(tper.com_id(), tper.com_id_ext()).await;
            }
        }

        // Close the TPer. This makes sure all session had the chance to terminate.
        // This call could hang the application if someone still holds a session or
        // if there is a bug in the protocol logic.
        if let Some(tper) = tper {
            let _ = tper.close().await;
        }
    }
}
