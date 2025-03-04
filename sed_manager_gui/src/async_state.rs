use std::rc::Rc;

use sed_manager_gui_elements::{ConfigureState, LockingRangeState};
use slint::{ComponentHandle as _, Model, SharedString};

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
        callback: impl AsyncFn(Backend) -> Result<(Vec<ui::DeviceIdentity>, Vec<ui::UnavailableDevice>), ui::ExtendedStatus>
            + 'static,
    ) {
        self.with(move |state| {
            let (backend, async_state, callback) = (self.backend.clone(), self.clone(), Rc::new(callback));

            state.on_list_devices(move || {
                let (backend, callback, async_state) = (backend.clone(), callback.clone(), async_state.clone());

                async_state.with(|state| state.wipe());
                let _ = slint::spawn_local(async move {
                    let result = callback(backend).await;
                    async_state.with(move |state| state.respond_list_devices(result));
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
            let (backend, async_state, callback) = (self.backend.clone(), self.clone(), Rc::new(callback));

            state.on_discover(move |device_idx: i32| {
                let (backend, callback, async_state) = (backend.clone(), callback.clone(), async_state.clone());
                async_state
                    .with(|state| state.respond_discover(device_idx as usize, Err(ui::ExtendedStatus::loading())));
                let _ = slint::spawn_local(async move {
                    let result = callback(backend, device_idx as usize).await;
                    async_state.with(move |state| state.respond_discover(device_idx as usize, result));
                });
            });
        });
    }

    pub fn on_cleanup_session(&self, callback: impl AsyncFn(Backend, usize) -> () + 'static) {
        self.with(move |state| {
            let (backend, callback) = (self.backend.clone(), Rc::new(callback));
            state.on_cleanup_session(move |device_idx: i32| {
                let (backend, callback) = (backend.clone(), callback.clone());
                let _ = slint::spawn_local(async move {
                    let _ = callback(backend, device_idx as usize).await;
                });
            });
        });
    }

    pub fn on_take_ownership(&self, callback: impl AsyncFn(Backend, usize, String) -> ui::ExtendedStatus + 'static) {
        self.with(move |state| {
            let (backend, async_state, callback) = (self.backend.clone(), self.clone(), Rc::new(callback));

            state.on_take_ownership(move |device_idx: i32, password: SharedString| {
                let (backend, callback, async_state) = (backend.clone(), callback.clone(), async_state.clone());

                async_state
                    .with(|state| state.respond_take_ownership(device_idx as usize, ui::ExtendedStatus::loading()));
                let _ = slint::spawn_local(async move {
                    let result = callback(backend, device_idx as usize, password.into()).await;
                    async_state.with(move |state| state.respond_take_ownership(device_idx as usize, result));
                });
            });
        });
    }

    pub fn on_activate_locking(
        &self,
        callback: impl AsyncFn(Backend, usize, String, String) -> ui::ExtendedStatus + 'static,
    ) {
        self.with(move |state| {
            let (backend, async_state, callback) = (self.backend.clone(), self.clone(), Rc::new(callback));

            state.on_activate_locking(move |device_idx: i32, sid_pw: SharedString, admin1_pw: SharedString| {
                let (backend, callback, async_state) = (backend.clone(), callback.clone(), async_state.clone());

                async_state
                    .with(|state| state.respond_activate_locking(device_idx as usize, ui::ExtendedStatus::loading()));
                let _ = slint::spawn_local(async move {
                    let result = callback(backend, device_idx as usize, sid_pw.into(), admin1_pw.into()).await;
                    async_state.with(move |state| state.respond_activate_locking(device_idx as usize, result));
                });
            });
        });
    }

    pub fn on_login_locking_ranges(
        &self,
        callback: impl AsyncFn(Backend, usize, String) -> ui::ExtendedStatus + 'static,
    ) {
        self.with(move |state| {
            let (backend, async_state, callback) = (self.backend.clone(), self.clone(), Rc::new(callback));

            state.on_login_locking_ranges(move |device_idx: i32, admin1_pw: SharedString| {
                let (backend, callback, async_state) = (backend.clone(), callback.clone(), async_state.clone());

                async_state.with(|state| {
                    state.respond_login_locking_ranges(device_idx as usize, ui::ExtendedStatus::loading())
                });
                let _ = slint::spawn_local(async move {
                    let result = callback(backend, device_idx as usize, admin1_pw.into()).await;
                    async_state.with(move |state| state.respond_login_locking_ranges(device_idx as usize, result));
                });
            });
        });
    }

    pub fn on_revert(
        &self,
        callback: impl AsyncFn(Backend, usize, bool, String, bool) -> ui::ExtendedStatus + 'static,
    ) {
        self.with(move |state| {
            let (backend, async_state, callback) = (self.backend.clone(), self.clone(), Rc::new(callback));

            state.on_revert(move |device_idx: i32, use_psid: bool, password: SharedString, revert_admin: bool| {
                let (backend, callback, async_state) = (backend.clone(), callback.clone(), async_state.clone());

                async_state.with(|state| state.respond_revert(device_idx as usize, ui::ExtendedStatus::loading()));
                let _ = slint::spawn_local(async move {
                    let result = callback(backend, device_idx as usize, use_psid, password.into(), revert_admin).await;
                    async_state.with(move |state| state.respond_revert(device_idx as usize, result));
                });
            });
        });
    }
}

pub trait StateExt {
    fn wipe(&self);

    fn respond_list_devices(
        &self,
        result: Result<(Vec<ui::DeviceIdentity>, Vec<ui::UnavailableDevice>), ui::ExtendedStatus>,
    );
    fn respond_discover(
        &self,
        device_idx: usize,
        result: Result<(ui::DeviceDiscovery, ui::ActivitySupport), ui::ExtendedStatus>,
    );
    fn respond_take_ownership(&self, device_idx: usize, status: ui::ExtendedStatus);
    fn respond_activate_locking(&self, device_idx: usize, status: ui::ExtendedStatus);
    fn respond_login_locking_ranges(&self, device_idx: usize, status: ui::ExtendedStatus);
    fn respond_revert(&self, device_idx: usize, status: ui::ExtendedStatus);
}

impl<'a> StateExt for ui::State<'a> {
    fn wipe(&self) {
        self.set_device_list_status(ui::ExtendedStatus::loading());
        self.set_tab_names(into_vec_model(vec![]));
        self.set_descriptions(into_vec_model(vec![]));
        self.set_unavailable_devices(into_vec_model(vec![]));
        self.set_configure(into_vec_model(vec![]));
        self.set_troubleshoot(into_vec_model(vec![]));
    }

    fn respond_list_devices(
        &self,
        result: Result<(Vec<ui::DeviceIdentity>, Vec<ui::UnavailableDevice>), ui::ExtendedStatus>,
    ) {
        let (identities, unavailable_devices) = match result {
            Ok(value) => value,
            Err(status) => {
                self.set_device_list_status(status);
                return;
            }
        };
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
        self.set_device_list_status(ui::ExtendedStatus::success());
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

    fn respond_take_ownership(&self, device_idx: usize, status: ui::ExtendedStatus) {
        respond_configure(self, device_idx, status);
    }

    fn respond_activate_locking(&self, device_idx: usize, status: ui::ExtendedStatus) {
        respond_configure(self, device_idx, status);
    }

    fn respond_login_locking_ranges(&self, device_idx: usize, status: ui::ExtendedStatus) {
        respond_configure(self, device_idx, status);
    }

    fn respond_revert(&self, device_idx: usize, status: ui::ExtendedStatus) {
        respond_configure(self, device_idx, status);
    }
}

fn respond_configure(state: &ui::State, device_idx: usize, status: ui::ExtendedStatus) {
    let configure = state.get_configure();
    let configure_vec = as_vec_model(&configure);
    if device_idx < configure_vec.row_count() {
        let value = ConfigureState::new(status, LockingRangeState::empty());
        configure_vec.set_row_data(device_idx, value);
    }
}
