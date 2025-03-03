use std::rc::Rc;
use std::sync::Arc;

use sed_manager::device::Device;
use sed_manager::messaging::discovery::Discovery;
use sed_manager::rpc::discover;
use sed_manager::tper::{Session, TPer};

use crate::device_list::{get_device_identity, DeviceList};
use crate::ui;
use crate::utility::{run_in_thread, PeekCell};

pub struct Backend {
    devices: Vec<Arc<dyn Device>>,
    discoveries: Vec<Option<Discovery>>,
    tpers: Vec<Option<Arc<TPer>>>,
    sessions: Vec<Option<Arc<Session>>>,
}

impl Backend {
    pub fn new() -> Self {
        Self { devices: Vec::new(), discoveries: Vec::new(), tpers: Vec::new(), sessions: Vec::new() }
    }

    pub async fn list_devices(this: Rc<PeekCell<Self>>) -> (Vec<ui::DeviceIdentity>, Vec<ui::UnavailableDevice>) {
        this.peek_mut(|this| {
            this.devices.clear();
            this.discoveries.clear();
            this.tpers.clear();
            this.sessions.clear();
        });
        let Ok(device_list) = DeviceList::query().await else {
            return (vec![], vec![]);
        };
        let mut identities = Vec::<ui::DeviceIdentity>::new();
        for device in &device_list.devices {
            identities.push(get_device_identity(device.clone()).await.into());
        }
        let unavailable_devices = device_list
            .unavailable_devices
            .into_iter()
            .map(|(path, error)| ui::UnavailableDevice::new(path, error.to_string()))
            .collect();
        this.peek_mut(move |this| {
            let num_devices = device_list.devices.len();
            this.devices = device_list.devices;
            this.discoveries = std::iter::repeat_with(|| None).take(num_devices).collect();
            this.tpers = std::iter::repeat_with(|| None).take(num_devices).collect();
            this.sessions = std::iter::repeat_with(|| None).take(num_devices).collect();
        });
        (identities, unavailable_devices)
    }

    pub async fn discover(
        this: Rc<PeekCell<Self>>,
        device_idx: usize,
    ) -> Result<(ui::DeviceDiscovery, ui::ActivitySupport), ui::ExtendedStatus> {
        let Some(device) = this.peek(|this| this.devices.get(device_idx).cloned()) else {
            return Err(ui::ExtendedStatus::error(format!("device {device_idx} not found (this is a bug)")));
        };
        let discovery = match run_in_thread(move || discover(&*device)).await {
            Ok(value) => value,
            Err(error) => return Err(ui::ExtendedStatus::error(error.to_string())),
        };
        let ui_discovery = ui::DeviceDiscovery::from_discovery(&discovery);
        let ui_activity_support = ui::ActivitySupport::from_discovery(&discovery);
        this.peek_mut(|this| this.discoveries.get_mut(device_idx).map(|opt| opt.replace(discovery)));
        Ok((ui_discovery, ui_activity_support))
    }
}
