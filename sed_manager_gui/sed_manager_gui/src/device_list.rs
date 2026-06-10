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
    pub tper: Option<Arc<Tper>>,
    pub session: Arc<Mutex<Session>>,
}
