use std::rc::Rc;

use sed_manager::messaging::com_id::ComIdState;
use sed_manager_gui_elements::{ConfigureState, ExtendedStatus, LockingRangeState, TroubleshootState};
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

    pub fn with<F: FnOnce(&ui::State) -> ()>(&self, f: F) {
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
        callback: impl AsyncFn(
                Backend,
                usize,
            )
                -> Result<(ui::DeviceDiscovery, ui::ActivitySupport, ui::DeviceGeometry), ui::ExtendedStatus>
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

    pub fn on_login_locking_admin(
        &self,
        callback: impl AsyncFn(Backend, usize, usize, String) -> ui::ExtendedStatus + 'static,
    ) {
        self.with(move |state| {
            let (backend, async_state, callback) = (self.backend.clone(), self.clone(), Rc::new(callback));

            state.on_login_locking_admin(move |device_idx: i32, admin_idx: i32, password: SharedString| {
                let (backend, callback, async_state) = (backend.clone(), callback.clone(), async_state.clone());

                async_state.with(|state| {
                    state.respond_login_locking_admin(device_idx as usize, ui::ExtendedStatus::loading())
                });
                let _ = slint::spawn_local(async move {
                    let result = callback(backend, device_idx as usize, admin_idx as usize, password.into()).await;
                    async_state.with(move |state| state.respond_login_locking_admin(device_idx as usize, result));
                });
            });
        });
    }

    pub fn on_list_locking_ranges(&self, callback: impl AsyncFn(Backend, usize, AsyncState<Backend>) + 'static) {
        self.with(move |state| {
            let (backend, async_state, callback) = (self.backend.clone(), self.clone(), Rc::new(callback));

            state.on_list_locking_ranges(move |device_idx: i32| {
                let (backend, callback, async_state) = (backend.clone(), callback.clone(), async_state.clone());
                let _ = slint::spawn_local(async move {
                    callback(backend, device_idx as usize, async_state).await;
                });
            });
        });
    }

    pub fn on_set_locking_range(
        &self,
        callback: impl AsyncFn(Backend, usize, usize, ui::LockingRange) -> Result<ui::LockingRange, ui::ExtendedStatus>
            + 'static,
    ) {
        self.with(move |state| {
            let (backend, async_state, callback) = (self.backend.clone(), self.clone(), Rc::new(callback));

            state.on_set_locking_range(move |device_idx: i32, range_idx: i32, properties: ui::LockingRange| {
                let (backend, callback, async_state) = (backend.clone(), callback.clone(), async_state.clone());
                async_state.with(|state| {
                    let loading = ui::ExtendedStatus::loading();
                    state.respond_locking_range_status(device_idx as usize, range_idx as usize, loading)
                });
                let _ = slint::spawn_local(async move {
                    let result = callback(backend, device_idx as usize, range_idx as usize, properties).await;
                    async_state.with(|state| {
                        state.respond_set_locking_range(device_idx as usize, range_idx as usize, result);
                    });
                });
            });
        });
    }

    pub fn on_erase_locking_range(
        &self,
        callback: impl AsyncFn(Backend, usize, usize) -> ui::ExtendedStatus + 'static,
    ) {
        self.with(move |state| {
            let (backend, async_state, callback) = (self.backend.clone(), self.clone(), Rc::new(callback));

            state.on_erase_locking_range(move |device_idx: i32, range_idx: i32| {
                let (backend, callback, async_state) = (backend.clone(), callback.clone(), async_state.clone());
                async_state.with(|state| {
                    let loading = ui::ExtendedStatus::loading();
                    state.respond_locking_range_status(device_idx as usize, range_idx as usize, loading)
                });
                let _ = slint::spawn_local(async move {
                    let result = callback(backend, device_idx as usize, range_idx as usize).await;
                    async_state.with(|state| {
                        state.respond_locking_range_status(device_idx as usize, range_idx as usize, result);
                    });
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

    pub fn on_reset_stack(&self, callback: impl AsyncFn(Backend, usize) -> ui::ExtendedStatus + 'static) {
        self.with(move |state| {
            let (backend, async_state, callback) = (self.backend.clone(), self.clone(), Rc::new(callback));

            state.on_reset_stack(move |device_idx: i32| {
                let (backend, callback, async_state) = (backend.clone(), callback.clone(), async_state.clone());

                async_state.with(|state| state.respond_reset_stack(device_idx as usize, ui::ExtendedStatus::loading()));
                let _ = slint::spawn_local(async move {
                    let result = callback(backend, device_idx as usize).await;
                    async_state.with(move |state| state.respond_reset_stack(device_idx as usize, result));
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
        result: Result<(ui::DeviceDiscovery, ui::ActivitySupport, ui::DeviceGeometry), ui::ExtendedStatus>,
    );
    fn respond_take_ownership(&self, device_idx: usize, status: ui::ExtendedStatus);
    fn respond_activate_locking(&self, device_idx: usize, status: ui::ExtendedStatus);
    fn respond_login_locking_admin(&self, device_idx: usize, status: ui::ExtendedStatus);
    fn push_locking_range(&self, device_idx: usize, name: String, range: ui::LockingRange);
    fn respond_set_locking_range(
        &self,
        device_idx: usize,
        range_idx: usize,
        result: Result<ui::LockingRange, ui::ExtendedStatus>,
    );
    fn respond_locking_range_status(&self, device_idx: usize, range_idx: usize, result: ui::ExtendedStatus);
    fn respond_revert(&self, device_idx: usize, status: ui::ExtendedStatus);
    fn respond_reset_stack(&self, device_idx: usize, result: ui::ExtendedStatus);
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
                    ui::DeviceGeometry::unknown(),
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
        result: Result<(ui::DeviceDiscovery, ui::ActivitySupport, ui::DeviceGeometry), ui::ExtendedStatus>,
    ) {
        let descriptions = self.get_descriptions();
        let Some(desc) = descriptions.row_data(device_idx) else {
            return;
        };
        let new_desc = match result {
            Ok((discovery, activity_support, geometry)) => ui::DeviceDescription {
                discovery_status: ui::ExtendedStatus::success(),
                discovery,
                activity_support,
                geometry,
                ..desc
            },
            Err(error) => ui::DeviceDescription { discovery_status: error, ..desc },
        };
        descriptions.set_row_data(device_idx, new_desc);
    }

    fn respond_take_ownership(&self, device_idx: usize, status: ui::ExtendedStatus) {
        respond_configure(self, device_idx, status);
    }

    fn respond_activate_locking(&self, device_idx: usize, status: ui::ExtendedStatus) {
        respond_configure(self, device_idx, status);
    }

    fn respond_login_locking_admin(&self, device_idx: usize, status: ui::ExtendedStatus) {
        respond_configure(self, device_idx, status);
    }

    fn push_locking_range(&self, device_idx: usize, name: String, range: ui::LockingRange) {
        let config = self.get_configure();
        let Some(dev_config) = config.row_data(device_idx) else {
            return;
        };
        as_vec_model(&dev_config.locking_ranges.names).push(name.into());
        as_vec_model(&dev_config.locking_ranges.properties).push(range.into());
        as_vec_model(&dev_config.locking_ranges.statuses).push(ui::ExtendedStatus::success());
        config.set_row_data(device_idx, dev_config);
    }

    fn respond_set_locking_range(
        &self,
        device_idx: usize,
        range_idx: usize,
        result: Result<ui::LockingRange, ui::ExtendedStatus>,
    ) {
        let config = self.get_configure();
        let Some(dev_config) = config.row_data(device_idx) else {
            return;
        };
        let prop_vec = as_vec_model(&dev_config.locking_ranges.properties);
        let status_vec = as_vec_model(&dev_config.locking_ranges.statuses);
        match result {
            Ok(new_props) => {
                if range_idx < prop_vec.row_count() {
                    prop_vec.set_row_data(range_idx, new_props);
                }
                if range_idx < status_vec.row_count() {
                    status_vec.set_row_data(range_idx, ExtendedStatus::success());
                }
            }
            Err(error) => {
                if range_idx < status_vec.row_count() {
                    status_vec.set_row_data(range_idx, error);
                }
            }
        }
    }

    fn respond_locking_range_status(&self, device_idx: usize, range_idx: usize, result: ui::ExtendedStatus) {
        let config = self.get_configure();
        let Some(dev_config) = config.row_data(device_idx) else {
            return;
        };
        let status_vec = as_vec_model(&dev_config.locking_ranges.statuses);
        if range_idx < status_vec.row_count() {
            status_vec.set_row_data(range_idx, result);
        }
    }

    fn respond_revert(&self, device_idx: usize, status: ui::ExtendedStatus) {
        respond_configure(self, device_idx, status);
    }

    fn respond_reset_stack(&self, device_idx: usize, result: ui::ExtendedStatus) {
        let troubleshoot = self.get_troubleshoot();
        if device_idx < troubleshoot.row_count() {
            let state = TroubleshootState::new(0, 0, ComIdState::Invalid, result);
            troubleshoot.set_row_data(device_idx, state);
        }
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
