use std::rc::Rc;

use slint::{ComponentHandle as _, Model};

use crate::{
    ui,
    utility::{as_vec_model, into_vec_model},
};

#[derive(Clone)]
pub struct AsyncState<Backend: Clone + 'static> {
    backend: Backend,
    window: slint::Weak<ui::AppWindow>,
}

impl<Backend: Clone + 'static> AsyncState<Backend> {
    pub fn new(backend: Backend, window: slint::Weak<ui::AppWindow>) -> Self {
        Self { backend, window }
    }

    fn with<F: FnOnce(&ui::State) -> ()>(&self, f: F) {
        if let Some(window) = self.window.upgrade() {
            let state = window.global::<ui::State>();
            f(&state);
        }
    }

    pub fn on_list_devices(
        &self,
        callback: impl AsyncFn(Backend) -> (Vec<ui::DeviceIdentity>, Vec<ui::UnavailableDevice>) + 'static,
    ) {
        self.with(move |state| {
            state.wipe();
            let backend = self.backend.clone();
            let async_state = self.clone();
            let callback = Rc::new(callback);
            state.on_list_devices(move || {
                let backend = backend.clone();
                let callback = callback.clone();
                let async_state = async_state.clone();
                let _ = slint::spawn_local(async move {
                    let (identities, unavailable_devices) = callback(backend).await;
                    async_state.with(move |state| state.respond_list_devices(identities, unavailable_devices));
                });
            });
        });
    }

    pub fn on_discover(
        &self,
        callback: impl AsyncFn(Backend, usize) -> Result<(ui::DeviceDiscovery, ui::ActivitySupport), ui::ExtendedStatus>
            + 'static,
    ) {
        self.with(move |state| {
            state.wipe();
            let backend = self.backend.clone();
            let async_state = self.clone();
            let callback = Rc::new(callback);
            state.on_discover(move |device_idx: i32| {
                let backend = backend.clone();
                let callback = callback.clone();
                let async_state = async_state.clone();
                let _ = slint::spawn_local(async move {
                    let result = callback(backend, device_idx as usize).await;
                    async_state.with(move |state| state.respond_discover(device_idx as usize, result));
                });
            });
        });
    }
}

pub trait StateExt {
    fn wipe(&self);

    fn respond_list_devices(
        &self,
        identities: Vec<ui::DeviceIdentity>,
        unavailable_devices: Vec<ui::UnavailableDevice>,
    );
    fn respond_discover(
        &self,
        device_idx: usize,
        result: Result<(ui::DeviceDiscovery, ui::ActivitySupport), ui::ExtendedStatus>,
    );
}

impl<'a> StateExt for ui::State<'a> {
    fn wipe(&self) {
        self.set_tab_names(into_vec_model(vec![]));
        self.set_descriptions(into_vec_model(vec![]));
        self.set_unavailable_devices(into_vec_model(vec![]));
        self.set_configure(into_vec_model(vec![]));
        self.set_troubleshoot(into_vec_model(vec![]));
    }

    fn respond_list_devices(
        &self,
        identities: Vec<ui::DeviceIdentity>,
        unavailable_devices: Vec<ui::UnavailableDevice>,
    ) {
        let num_devices = identities.len();
        let mut tabs: Vec<_> = identities.iter().map(|identity| identity.name.clone()).collect();
        if !unavailable_devices.is_empty() {
            tabs.push("Unavailable devices".into());
        }
        let descriptions: Vec<_> = identities
            .into_iter()
            .map(|identity| {
                ui::DeviceDescription::new(
                    identity,
                    ui::ExtendedStatus::loading(),
                    ui::DeviceDiscovery::empty(),
                    ui::ActivitySupport::none(),
                )
            })
            .collect();
        let configures: Vec<_> = std::iter::repeat_with(|| ui::ConfigureState::empty()).take(num_devices).collect();
        let troubleshoots: Vec<_> =
            std::iter::repeat_with(|| ui::TroubleshootState::empty()).take(num_devices).collect();
        self.set_descriptions(into_vec_model(descriptions));
        self.set_unavailable_devices(into_vec_model(unavailable_devices));
        self.set_tab_names(into_vec_model(tabs));
        self.set_configure(into_vec_model(configures));
        self.set_troubleshoot(into_vec_model(troubleshoots));
    }

    fn respond_discover(
        &self,
        device_idx: usize,
        result: Result<(ui::DeviceDiscovery, ui::ActivitySupport), ui::ExtendedStatus>,
    ) {
        let descriptions = self.get_descriptions();
        let description_vec = as_vec_model(&descriptions);
        let Some(desc) = description_vec.row_data(device_idx) else {
            return;
        };
        let new_desc = match result {
            Ok((discovery, activity_support)) => ui::DeviceDescription {
                discovery_status: ui::ExtendedStatus::success(),
                discovery,
                activity_support,
                ..desc
            },
            Err(error) => ui::DeviceDescription { discovery_status: error, ..desc },
        };
        description_vec.set_row_data(device_idx, new_desc);
    }
}
