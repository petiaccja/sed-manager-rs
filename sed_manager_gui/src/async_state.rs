use std::rc::Rc;

use slint::{ComponentHandle as _, SharedString};

use crate::state_ext::StateExt;
use crate::ui;

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

    pub fn on_list_locking_users(&self, callback: impl AsyncFn(Backend, usize, AsyncState<Backend>) + 'static) {
        self.with(move |state| {
            let (backend, async_state, callback) = (self.backend.clone(), self.clone(), Rc::new(callback));

            state.on_list_locking_users(move |device_idx: i32| {
                let (backend, callback, async_state) = (backend.clone(), callback.clone(), async_state.clone());
                let _ = slint::spawn_local(async move {
                    callback(backend, device_idx as usize, async_state).await;
                });
            });
        });
    }

    pub fn on_set_locking_user_enabled(
        &self,
        callback: impl AsyncFn(Backend, usize, usize, bool) -> Result<bool, ui::ExtendedStatus> + 'static,
    ) {
        self.with(move |state| {
            let (backend, async_state, callback) = (self.backend.clone(), self.clone(), Rc::new(callback));

            state.on_set_locking_user_enabled(move |device_idx: i32, user_idx: i32, enabled: bool| {
                let (backend, callback, async_state) = (backend.clone(), callback.clone(), async_state.clone());
                async_state.with(|state| {
                    let loading = ui::ExtendedStatus::loading();
                    state.respond_locking_user_status(device_idx as usize, user_idx as usize, loading)
                });
                let _ = slint::spawn_local(async move {
                    let result = callback(backend, device_idx as usize, user_idx as usize, enabled).await;
                    async_state.with(|state| {
                        state.respond_set_locking_user_enabled(device_idx as usize, user_idx as usize, result);
                    });
                });
            });
        });
    }

    pub fn on_set_locking_user_name(
        &self,
        callback: impl AsyncFn(Backend, usize, usize, SharedString) -> Result<String, ui::ExtendedStatus> + 'static,
    ) {
        self.with(move |state| {
            let (backend, async_state, callback) = (self.backend.clone(), self.clone(), Rc::new(callback));

            state.on_set_locking_user_name(move |device_idx: i32, range_idx: i32, name: SharedString| {
                let (backend, callback, async_state) = (backend.clone(), callback.clone(), async_state.clone());
                async_state.with(|state| {
                    let loading = ui::ExtendedStatus::loading();
                    state.respond_locking_user_status(device_idx as usize, range_idx as usize, loading)
                });
                let _ = slint::spawn_local(async move {
                    let result = callback(backend, device_idx as usize, range_idx as usize, name).await;
                    async_state.with(|state| {
                        state.respond_set_locking_user_name(device_idx as usize, range_idx as usize, result);
                    });
                });
            });
        });
    }

    pub fn on_set_locking_user_password(
        &self,
        callback: impl AsyncFn(Backend, usize, usize, SharedString) -> ui::ExtendedStatus + 'static,
    ) {
        self.with(move |state| {
            let (backend, async_state, callback) = (self.backend.clone(), self.clone(), Rc::new(callback));

            state.on_set_locking_user_password(move |device_idx: i32, range_idx: i32, name: SharedString| {
                let (backend, callback, async_state) = (backend.clone(), callback.clone(), async_state.clone());
                async_state.with(|state| {
                    let loading = ui::ExtendedStatus::loading();
                    state.respond_locking_user_status(device_idx as usize, range_idx as usize, loading)
                });
                let _ = slint::spawn_local(async move {
                    let result = callback(backend, device_idx as usize, range_idx as usize, name).await;
                    async_state.with(|state| {
                        state.respond_locking_user_status(device_idx as usize, range_idx as usize, result);
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
