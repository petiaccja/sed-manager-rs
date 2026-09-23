//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    rc::{Rc, Weak},
    sync::Arc,
};

use async_lock::RwLock;
use sed_async::PolyRuntime;
use sed_manager::{Device, Error, Host, Spec};
use sed_manager_gui_slint as ui;
use sed_packet::{MaxBytes, com_id::ComIdState};
use sed_spec::{
    methods::MethodStatus,
    objects::{Authority, AuthorityRef, SecurityProviderRef},
    preconfig::{core::shared::authority::ANYBODY, opal_2::locking},
};
use sed_tper::{Error as TperError, PropertiesChanged};
use slint::{
    CloseRequestResponse, ComponentHandle, Model, ModelExt as _, ModelRc, SharedString, ToSharedString, VecModel,
    quit_event_loop, spawn_local,
};
use tracing::{error, instrument};

use crate::{
    command::{Command, ExpectInEventLoop},
    device_list::{DeviceEntry, DeviceList},
    session::Session,
    toast::ToastQueue,
    ui_conv::{CombinedProperties, IntoUi, IntoUiName, MbrDesc, TryFromUi as _},
    ui_ext::{DeviceExt as _, DiscoveryExt, StackStatusExt},
};

pub struct App {
    ui: ui::MainWindow,
    device_list: Arc<RwLock<DeviceList>>,
    toast_queue: Rc<ToastQueue>,
    host: Arc<Host>,
    runtime: Arc<PolyRuntime>,
}

impl App {
    pub fn new(
        ui: ui::MainWindow,
        notification_queue: Rc<ToastQueue>,
        host: Arc<Host>,
        runtime: Arc<PolyRuntime>,
    ) -> Rc<Self> {
        let device_list = DeviceList::default();
        let device_list_ui = device_list.ui.inner();

        let view_model = Rc::from(Self {
            ui: ui.clone_strong(),
            toast_queue: notification_queue,
            device_list: Arc::new(RwLock::new(device_list)),
            host,
            runtime,
        });

        // Callbacks
        {
            let view_model = view_model.clone();
            ui.on_scan(move || view_model.clone().scan(false));
        }
        {
            let view_model = view_model.clone();
            ui.on_close_device(move |path| view_model.clone().close_device(path));
        }
        {
            let view_model = view_model.clone();
            ui.on_take_owneship(move |path, password| {
                view_model.clone().take_ownership(path.to_string().into(), password)
            });
        }
        {
            let view_model = view_model.clone();
            ui.on_activate_locking(move |path, password| {
                view_model.clone().activate_locking(path.to_string().into(), password)
            });
        }
        {
            let view_model = view_model.clone();
            ui.on_change_password(move |path, sp, authority, current_password, new_password| {
                view_model.clone().change_password(
                    path.to_string().into(),
                    sp,
                    authority,
                    current_password,
                    new_password,
                )
            });
        }
        {
            let view_model = view_model.clone();
            ui.on_list_admin_authorities(move |path, silent| {
                view_model.clone().list_admin_authorities(path.to_string().into(), silent)
            });
        }
        {
            let view_model = view_model.clone();
            ui.on_list_locking_authorities(move |path, silent| {
                view_model.clone().list_locking_authorities(path.to_string().into(), silent)
            });
        }
        {
            let view_model = view_model.clone();
            ui.on_login(move |path, authority, password| {
                view_model.clone().login(path.to_string().into(), authority, password)
            });
        }
        {
            let view_model = view_model.clone();
            ui.on_logout(move |path| view_model.clone().logout(path.to_string().into()));
        }
        {
            let view_model = view_model.clone();
            ui.on_revert_device(move |path, scope, authority, password| {
                view_model.clone().revert_device(path.to_string().into(), scope, authority, password)
            });
        }
        {
            let view_model = view_model.clone();
            ui.on_query_stack_status(move |path| view_model.clone().query_stack_status(path.to_string().into(), false));
        }
        {
            let view_model = view_model.clone();
            ui.on_reset_stack(move |path| view_model.clone().reset_stack(path.to_string().into()));
        }
        {
            let view_model = view_model.clone();
            ui.window().on_close_requested(move || {
                view_model.clone().quit();
                CloseRequestResponse::KeepWindowShown
            });
        }

        // Device model
        {
            let sorted = device_list_ui.sort_by(|lhs, rhs| {
                fn key(identity: &ui::Identity) -> (bool, &SharedString, &SharedString) {
                    (!identity.security_commands, &identity.name, &identity.serial)
                }
                key(&lhs.identity).cmp(&key(&rhs.identity))
            });
            ui.set_devices(ModelRc::from(Rc::from(sorted)));
        }

        view_model
    }

    fn command(&self) -> Command {
        Command::new(self.runtime.clone(), self.device_list.clone())
    }

    #[instrument(skip(self, silent))]
    pub fn scan(self: Rc<Self>, silent: bool) {
        self.ui.set_scan_outcome(ui::Outcome::Pending);
        let host = self.host.clone();
        self.command()
            .on_device_list(async move |device_list| {
                let mut new_paths: HashSet<_> = host.list_devices().await?.into_iter().collect();

                // The paths must be losslessly converted to Slint string because
                // they are used as HashMap keys.
                let non_unicode = retain_unicode(&mut new_paths);

                // Insert virtual device in debug mode, or when explicitly requested (e.g. by tests).
                #[cfg(not(any(debug_assertions, feature = "virtual_device")))]
                new_paths.remove(Path::new(sed_virtual_device::VIRTUAL_DEVICE_PATH));

                let removed: HashSet<_> =
                    device_list.backend.extract_if(|path, _| !new_paths.contains(path)).map(|(path, _)| path).collect();

                let mut added = Vec::new();
                for path in new_paths {
                    if !device_list.backend.contains_key(&path) {
                        device_list.backend.insert(path.clone(), Default::default());
                        added.push(path);
                    }
                }

                Ok::<_, Error>((added, removed, non_unicode))
            })
            .display(move |device_list, result| {
                self.ui.set_scan_outcome(ui::Outcome::Idle);
                match result {
                    Ok((added, removed, non_unicode)) => {
                        for path in added {
                            let path_str = path.to_string_lossy().to_shared_string();
                            device_list.ui.insert(
                                path.clone(),
                                ui::Device {
                                    identity: ui::Identity {
                                        path: path_str,
                                        status: ui::Status { outcome: ui::Outcome::Idle, ..Default::default() },
                                        ..Default::default()
                                    },
                                    discovery: ui::Discovery {
                                        status: ui::Status { outcome: ui::Outcome::Idle, ..Default::default() },
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                },
                            );

                            self.clone().open(path);
                        }

                        for path in removed {
                            device_list.ui.remove(&path);
                        }

                        for path in non_unicode {
                            let path = path.to_string_lossy();
                            self.toast_queue.warning(
                                "Device ignored".into(),
                                format!(
                                    "Device paths must be valid unicode strings. The device {path} will be ignored"
                                ),
                            );
                        }

                        if !silent {
                            self.toast_queue.success("Device list updated".into(), "".into());
                        }
                    }
                    Err(err) => self.toast_queue.error("Could not update device list".into(), err.to_string()),
                }
            })
            .run();
    }

    #[instrument(skip(self))]
    fn close_device(self: Rc<Self>, path: SharedString) {
        self.ui.set_scan_outcome(ui::Outcome::Pending);
        let path = PathBuf::from(String::from(path));

        let app = self.clone();
        self.command()
            .on_device_list(async move |device_list| {
                device_list.ui.remove(&path);
                if let Some(device) = device_list.backend.remove(&path) {
                    let mut device = device.write().await;
                    device.close().await;
                }
            })
            .display(move |_device_list, _| {
                app.ui.set_scan_outcome(ui::Outcome::Idle);
            })
            .run();
    }

    #[instrument(skip(self))]
    fn open(self: Rc<Self>, path: PathBuf) {
        let app = self.clone();
        let path_ = path.clone();
        let host = self.host.clone();
        self.command()
            .on_device_entry(path.clone(), async move |device_entry: &mut DeviceEntry| {
                match host.open_device(path_).await {
                    Ok(device) => {
                        let storage_device = device.storage_device();
                        let capabilities = device.capabilities().ok();
                        let properties_changed = device.properties_changed().ok();
                        device_entry.device = Some(device);
                        Ok((storage_device, capabilities, properties_changed))
                    }
                    Err(err) => Err(err),
                }
            })
            .display(move |mut ui_device, result| match result {
                Ok((storage_device, capabilities, properties_changed)) => {
                    if storage_device.is_security_supported() {
                        app.clone().discover(path.clone());
                    }
                    if let (Some(capabilities), Some(properties_changed)) = (capabilities, properties_changed) {
                        let combined_properties =
                            CombinedProperties { host: capabilities, device: None, connection: None };
                        let status = ui_device.stack_status.clone().with_protocol(combined_properties.into_ui());
                        spawn_local(Self::listen_connection_changed(
                            Rc::downgrade(&self),
                            path.clone(),
                            properties_changed,
                        ))
                        .expect_in_event_loop();
                        self.clone().query_stack_status(path.clone(), true);
                        self.clone().list_security_providers(path.clone());
                        ui_device.stack_status = status;
                    }
                    ui_device.identity = storage_device.into_ui();
                    ui_device
                }
                Err(err) => {
                    let identity = ui_device.identity.clone();
                    ui_device.with_identity(ui::Identity {
                        status: ui::Status { outcome: ui::Outcome::Error, message: err.to_shared_string() },
                        ..identity
                    })
                }
            })
            .run();
    }

    #[instrument(skip(self))]
    fn discover(self: Rc<Self>, path: PathBuf) {
        self.command()
            .on_device(path.clone(), async move |device: &Device| match device.discover().await {
                Ok(mut discovery) => {
                    Spec::sort(&mut discovery);
                    Some(Ok(discovery))
                }
                Err(err) => Some(Err(err)),
            })
            .display(move |ui_device, result| match result {
                Some(Ok(discovery)) => {
                    let (ui_config, ui_discovery) = discovery.into_ui();
                    ui_device.with_discovery(ui_discovery).with_config(ui_config)
                }
                Some(Err(err)) => ui_device.with_discovery(ui::Discovery::error(err.to_string())),
                None => ui_device,
            })
            .run();
    }

    #[instrument(skip(self, silent))]
    fn query_stack_status(self: Rc<Self>, path: PathBuf, silent: bool) {
        self.command()
            .on_device(path.clone(), async |device: &Device| -> Result<_, Error> {
                Ok((device.com_id()?, device.com_id_ext()?, device.query_com_id_state().await))
            })
            .display(move |ui_device, result| {
                let ui_status = match result {
                    Ok((com_id, com_id_ext, Ok(state))) => {
                        let good = [ComIdState::Issued, ComIdState::Associated].contains(&state);
                        if !silent {
                            self.toast_queue.success("Stack status updated".into(), "".to_string());
                        }
                        let status = state.to_shared_string();
                        ui::ComIdStatus { com_id: com_id.into(), com_id_ext: com_id_ext.into(), status, good }
                    }
                    Ok((com_id, com_id_ext, Err(err))) => {
                        if !silent {
                            self.toast_queue.error("Could not update stack status".into(), err.to_string());
                        }
                        let status = err.to_shared_string();
                        ui::ComIdStatus { com_id: com_id.into(), com_id_ext: com_id_ext.into(), good: false, status }
                    }
                    Err(err) => {
                        if !silent {
                            self.toast_queue.error("Could not update stack status".into(), err.to_string());
                        }
                        let status = err.to_shared_string();
                        ui::ComIdStatus { good: false, status, ..Default::default() }
                    }
                };
                let status = ui_device.stack_status.clone().with_com_id(ui_status);
                ui_device.with_stack_status(status)
            })
            .run();
    }

    #[instrument(skip(self))]
    fn list_security_providers(self: Rc<Self>, path: PathBuf) {
        self.command()
            .on_device(path.clone(), async |device: &Device| {
                Spec::try_from(device.discover().await?).map_err(|_| Error::NoSscAvailable)
            })
            .display(move |mut ui_device, spec| match spec {
                Ok(spec) => {
                    let features = &spec.discovery.feature_descriptors;
                    ui_device.admin_sp.uid = spec.admin.uid.into_ui();
                    ui_device.admin_sp.name = spec.admin.uid.into_ui_name(features, Some(spec.admin.uid));
                    if let Some(locking_sp) = spec.locking {
                        ui_device.locking_sp.uid = locking_sp.uid.into_ui();
                        ui_device.locking_sp.name = locking_sp.uid.into_ui_name(features, Some(spec.admin.uid))
                    }
                    ui_device
                }
                Err(err) => {
                    self.toast_queue.error("Can not list SPs".into(), err.to_string());
                    ui_device
                }
            })
            .run();
    }

    #[instrument(skip(self))]
    fn list_admin_authorities(self: Rc<Self>, path: PathBuf, silent: bool) {
        self.command()
            .on_session(path.clone(), async |device: &Device, session: &mut Session| {
                let setup_session = session.start_setup_session(device).await?;
                let sp_ref = setup_session.spec().admin.uid;
                setup_session.list_authorities(sp_ref).await.map(|auths| (auths, sp_ref))
            })
            .display(move |mut ui_device, spec, result| match result {
                Ok((authorities, sp_ref)) => {
                    let (authorities, individual_authority_names, individual_authority_uids) =
                        display_authorities(spec.clone(), &authorities, Some(sp_ref));

                    ui_device.admin_sp.authorities = authorities.into();
                    ui_device.admin_sp.individual_authority_names = Rc::from(individual_authority_names).into();
                    ui_device.admin_sp.individual_authority_uids = Rc::from(individual_authority_uids).into();

                    if !silent {
                        self.toast_queue.success("Admin authorities updated".into(), String::new());
                    }
                    ui_device
                }
                Err(err) => {
                    self.toast_queue.error("Failed to update admin authorities".into(), err.to_string());
                    ui_device
                }
            })
            .run();
    }

    #[instrument(skip(self))]
    fn list_locking_authorities(self: Rc<Self>, path: PathBuf, silent: bool) {
        self.command()
            .on_session(path.clone(), async |device: &Device, session: &mut Session| {
                let setup_session = session.start_setup_session(device).await?;
                if let Some(locking_sp) = &setup_session.spec().locking {
                    setup_session.list_authorities(locking_sp.uid).await.map(|auths| (auths, Some(locking_sp.uid)))
                } else {
                    Ok((vec![], None))
                }
            })
            .display(move |mut ui_device, spec, result| match result {
                Ok((authorities, sp_ref)) => {
                    let (authorities, individual_authority_names, individual_authority_uids) =
                        display_authorities(spec.clone(), &authorities, sp_ref);

                    ui_device.locking_sp.authorities = authorities.into();
                    ui_device.locking_sp.individual_authority_names = Rc::from(individual_authority_names).into();
                    ui_device.locking_sp.individual_authority_uids = Rc::from(individual_authority_uids).into();

                    if !silent {
                        self.toast_queue.success("Locking authorities updated".into(), String::new());
                    }
                    ui_device
                }
                Err(err) => {
                    self.toast_queue.error("Failed to update locking authorities".into(), err.to_string());
                    ui_device
                }
            })
            .run();
    }

    #[instrument(skip(self, password))]
    fn login(self: Rc<Self>, path: PathBuf, authority: ui::Uid, password: SharedString) {
        let Some(password) = self.try_convert_password(password.as_str()) else {
            return;
        };
        let Ok(authority) = AuthorityRef::try_from_ui(&authority) else {
            error!(authority_ref = &authority.value, "invalid authority reference");
            self.toast_queue.error(
                "Invalid authority reference".into(),
                "The UID is not an authority reference. Please report this bug.".into(),
            );
            return;
        };

        self.command()
            .on_session(path.clone(), async move |device: &Device, session: &mut Session| {
                session.start_locking_config_session(device, authority, Some(password)).await.map(|_| ())
            })
            .display(move |ui_device, _spec, result| match result {
                Ok(_) => {
                    self.clone().list_locking_config_authorities(path.clone(), true);
                    self.clone().list_locking_config_ranges(path.clone(), true);
                    self.clone().get_mbr(path.clone(), true);
                    ui_device
                }
                Err(err) => {
                    self.toast_queue.error("Login failed".into(), err.to_string());
                    ui_device
                }
            })
            .run();
    }

    #[instrument(skip(self))]
    fn list_locking_config_authorities(self: Rc<Self>, path: PathBuf, silent: bool) {
        self.command()
            .on_session(path.clone(), async |_device: &Device, session: &mut Session| {
                let Session::LockingConfig(locking_config_session) = &*session else {
                    return None;
                };
                Some(locking_config_session.get_authorities().await)
            })
            .display(move |mut ui_device, spec, result| match result {
                Some(Ok(authorities)) => {
                    let sp_ref = spec.and_then(|spec| spec.locking.as_ref()).map(|sp| sp.uid);
                    let (authorities, individual_authority_names, individual_authority_uids) =
                        display_authorities(spec, &authorities, sp_ref);

                    ui_device.locking_sp.authorities = authorities.into();
                    ui_device.locking_sp.individual_authority_names = Rc::from(individual_authority_names).into();
                    ui_device.locking_sp.individual_authority_uids = Rc::from(individual_authority_uids).into();

                    if !silent {
                        self.toast_queue.success("Locking authorities updated".into(), String::new());
                    }
                    ui_device
                }
                Some(Err(err)) => {
                    self.toast_queue.error("Failed to update locking authorities".into(), err.to_string());
                    ui_device
                }
                _ => ui_device,
            })
            .run();
    }

    #[instrument(skip(self))]
    fn list_locking_config_ranges(self: Rc<Self>, path: PathBuf, silent: bool) {
        self.command()
            .on_session(path.clone(), async |_device: &Device, session: &mut Session| {
                let Session::LockingConfig(locking_config_session) = &*session else {
                    return None;
                };
                Some(locking_config_session.get_locking_ranges().await)
            })
            .display(move |mut ui_device, spec, result| match result {
                Some(Ok(ranges)) => {
                    let features = spec.map(|spec| spec.discovery.feature_descriptors.as_slice()).unwrap_or(&[]);
                    let sp_ref = spec.and_then(|spec| spec.locking.as_ref()).map(|sp| sp.uid);
                    let ranges: Vec<_> = ranges.iter().map(|range| range.into_ui_name(features, sp_ref)).collect();
                    ui_device.locking_sp.locking_ranges = Rc::from(VecModel::from(ranges)).into();

                    if !silent {
                        self.toast_queue.success("Locking ranges updated".into(), String::new());
                    }
                    ui_device
                }
                Some(Err(err)) => {
                    self.toast_queue.error("Failed to update locking ranges".into(), err.to_string());
                    ui_device
                }
                _ => ui_device,
            })
            .run();
    }

    #[instrument(skip(self))]
    fn get_mbr(self: Rc<Self>, path: PathBuf, silent: bool) {
        self.command()
            .on_session(path.clone(), async |_device: &Device, session: &mut Session| {
                let Session::LockingConfig(locking_config_session) = &*session else {
                    return None;
                };

                let size = match locking_config_session.get_mbr_size().await {
                    Ok(size) => size,
                    Err(Error::TperError(TperError::MethodCallFailed(MethodStatus::InvalidParameter))) => {
                        return Some(Ok(MbrDesc { supported: false, size: None, control: None }));
                    }
                    Err(err) => return Some(Err(err)),
                };
                let control = match locking_config_session.get_mbr_control().await {
                    Ok(control) => control,
                    Err(err) => return Some(Err(err)),
                };
                Some(Ok(MbrDesc { supported: true, size: Some(size), control: Some(control) }))
            })
            .display(move |mut ui_device, _spec, result| match result {
                Some(Ok(mbr)) => {
                    ui_device.locking_sp.mbr = mbr.into_ui();

                    if !silent {
                        self.toast_queue.success("Shadow MBR status updated".into(), String::new());
                    }
                    ui_device
                }
                Some(Err(err)) => {
                    self.toast_queue.error("Failed to update shadow MBR status".into(), err.to_string());
                    ui_device
                }
                _ => ui_device,
            })
            .run();
    }

    #[instrument(skip(self))]
    fn logout(self: Rc<Self>, path: PathBuf) {
        self.command()
            .on_session(path.clone(), async move |device: &Device, session: &mut Session| {
                if matches!(*session, Session::LockingConfig(_)) {
                    if let Err(_) = session.close().await {
                        let _ = device.stack_reset().await;
                    }
                }
            })
            .display(move |ui_device, _spec, _result| ui_device)
            .run();
    }

    #[instrument(skip(self))]
    fn reset_stack(self: Rc<Self>, path: PathBuf) {
        let app = self.clone();
        self.command()
            .on_device(path.clone(), async |device: &Device| device.stack_reset().await)
            .display(move |ui_device, result| {
                match result {
                    Ok(_) => app.toast_queue.success("Stack has been reset".into(), "".into()),
                    Err(err) => app.toast_queue.error("Could not reset stack".into(), err.to_string()),
                };
                ui_device
            })
            .run();
    }

    #[instrument(skip(self, password))]
    fn take_ownership(self: Rc<Self>, path: PathBuf, password: SharedString) {
        let Some(password) = self.try_convert_password(password.as_str()) else {
            return;
        };

        self.command()
            .on_session(path.clone(), async move |device: &Device, session: &mut Session| {
                let sid_session = session.start_setup_session(device).await?;
                sid_session.take_owneship(password).await
            })
            .display(move |ui_device, _, result| {
                match result {
                    Ok(_) => {
                        self.toast_queue.success("Taken ownership".into(), "".into());
                        self.clone().discover(path.clone());
                    }
                    Err(err) => self.toast_queue.error("Could not take ownership".into(), err.to_string()),
                };
                ui_device
            })
            .run();
    }

    #[instrument(skip(self, password))]
    fn activate_locking(self: Rc<Self>, path: PathBuf, password: SharedString) {
        let Some(password) = self.try_convert_password(password.as_str()) else {
            return;
        };

        self.command()
            .on_session(path.clone(), async move |device: &Device, session: &mut Session| {
                let sid_session = session.start_setup_session(device).await?;
                sid_session.activate_secondary_sp(password).await
            })
            .display(move |ui_device, _, result| {
                match result {
                    Ok(_) => {
                        self.toast_queue.success("Locking activated".into(), "".into());
                        self.clone().discover(path.clone());
                    }
                    Err(err) => self.toast_queue.error("Could not activate locking".into(), err.to_string()),
                };
                ui_device
            })
            .run();
    }

    #[instrument(skip(self, current_password, new_password))]
    fn change_password(
        self: Rc<Self>,
        path: PathBuf,
        sp: ui::Uid,
        authority: ui::Uid,
        current_password: SharedString,
        new_password: SharedString,
    ) {
        let Some(current_password) = self.try_convert_password(current_password.as_str()) else {
            return;
        };
        let Some(new_password) = self.try_convert_password(new_password.as_str()) else {
            return;
        };
        let Ok(sp) = SecurityProviderRef::try_from_ui(&sp) else {
            error!(sp_ref = &sp.value, "invalid SP reference");
            self.toast_queue
                .error("Invalid SP reference".into(), "The UID is not an SP reference. Please report this bug.".into());
            return;
        };
        let Ok(authority) = AuthorityRef::try_from_ui(&authority) else {
            error!(authority_ref = &authority.value, "invalid authority reference");
            self.toast_queue.error(
                "Invalid authority reference".into(),
                "The UID is not an authority reference. Please report this bug.".into(),
            );
            return;
        };

        self.command()
            .on_session(path.clone(), async move |device: &Device, session: &mut Session| {
                let sid_session = session.start_setup_session(device).await?;
                sid_session.change_password(sp, authority, current_password, new_password).await
            })
            .display(move |ui_device, _, result| {
                match result {
                    Ok(_) => self.toast_queue.success("Password changed".into(), "".into()),
                    Err(err) => self.toast_queue.error("Could not change password".into(), err.to_string()),
                };
                ui_device
            })
            .run();
    }

    #[instrument(skip(self, password))]
    fn revert_device(
        self: Rc<Self>,
        path: PathBuf,
        scope: ui::RevertScope,
        authority: ui::RevertAuthority,
        password: SharedString,
    ) {
        let Some(password) = self.try_convert_password(password.as_str()) else {
            return;
        };

        self.command()
            .on_session(path.clone(), async move |device: &Device, session: &mut Session| {
                let sid_session = session.start_setup_session(device).await?;
                let authority = match authority {
                    ui::RevertAuthority::Sid => sid_session.spec().admin.authorities.sid,
                    ui::RevertAuthority::Psid => sid_session.spec().admin.authorities.psid,
                };
                match scope {
                    ui::RevertScope::Locking => sid_session.revert_secondary_sp(password).await,
                    ui::RevertScope::Everything => sid_session.revert_tper(authority, password).await,
                }
            })
            .display(move |ui_device, _, result| {
                match result {
                    Ok(_) => {
                        self.toast_queue.success("Reverted device successfully".into(), "".into());
                        self.clone().discover(path.clone());
                    }
                    Err(err) => self.toast_queue.error("Reverting device failed".into(), err.to_string()),
                };
                ui_device
            })
            .run();
    }

    #[instrument(skip(self))]
    fn quit(&self) {
        self.ui.set_is_quitting(true);

        self.command()
            .on_device_list(async move |device_list| {
                // This should be done concurrently for all devices.
                // Unfortunately we don't have access to the runtime here, but
                // if the devices are well-behaved, this should be quick.
                for device in device_list.backend.values_mut() {
                    let mut device = device.write().await;
                    device.close().await;
                }
            })
            .display(move |_device_list, _| {
                // Once all device are closed we can quit the event loop.
                let _ = quit_event_loop();
            })
            .run();
    }

    fn try_convert_password(&self, password: &str) -> Option<MaxBytes<32>> {
        let byte_password: MaxBytes<32> = password.as_bytes().into();
        if byte_password.len() == password.as_bytes().len() {
            Some(byte_password)
        } else {
            self.toast_queue.error(
                "Password too long".into(),
                "The password cannot be longer than 32 bytes (32 Latin characters)".into(),
            );
            None
        }
    }

    async fn listen_connection_changed(
        self_: Weak<Self>,
        path: PathBuf,
        mut event: async_broadcast::Receiver<PropertiesChanged>,
    ) {
        loop {
            match event.recv().await {
                Ok(value) => {
                    let Some(app) = self_.upgrade() else { break };
                    app.command()
                        .on_device(path.clone(), async |device: &Device| device.capabilities())
                        .display(move |ui_device, host_capabilities| {
                            if let Ok(host_capabilities) = host_capabilities {
                                let combined = CombinedProperties {
                                    host: host_capabilities,
                                    device: Some(value.remote_properties),
                                    connection: Some(value.connection_properties),
                                };
                                let status = ui_device.stack_status.clone().with_protocol(combined.into_ui());
                                ui_device.with_stack_status(status)
                            } else {
                                ui_device
                            }
                        })
                        .run();
                }
                Err(async_broadcast::RecvError::Overflowed(_)) => (),
                Err(async_broadcast::RecvError::Closed) => break,
            }
        }
    }
}

fn display_authorities(
    spec: Option<&Spec>,
    authorities: &[Authority],
    sp_ref: Option<SecurityProviderRef>,
) -> (
    Rc<VecModel<ui::Authority>>,
    impl Model<Data = SharedString> + use<>,
    impl Model<Data = ui::Uid> + use<>,
) {
    // Convert the authority structures to their UI counterparts.
    let features = spec.map(|spec| spec.discovery.feature_descriptors.as_slice()).unwrap_or(&[]);
    let authorities: Vec<_> = authorities.iter().map(|auth| auth.into_ui_name(features, sp_ref)).collect();
    let authorities = Rc::from(VecModel::from(authorities));

    // Map and filter user names and UIDs for the password change
    // activity. TODO: move this into Slint when feature is available.
    const NON_PASSWORD_AUTHORITIES: [AuthorityRef; 3] = [
        ANYBODY,
        locking::authority::ADMINS,
        locking::authority::USERS,
    ];
    let individual_authorities = Rc::from(authorities.clone().filter(|auth| {
        let auth_ref = AuthorityRef::try_from(auth.uid.value.cast_unsigned());
        !auth.is_class && auth.enabled && auth_ref.is_ok_and(|auth| !NON_PASSWORD_AUTHORITIES.contains(&auth))
    }));
    let individual_authority_names = individual_authorities.clone().map(|auth| auth.name);
    let individual_authority_uids = individual_authorities.clone().map(|auth| auth.uid);
    (authorities, individual_authority_names, individual_authority_uids)
}

fn retain_unicode(paths: &mut HashSet<PathBuf>) -> Vec<PathBuf> {
    fn is_unicode(path: &Path) -> bool {
        path.to_str().is_some()
    }

    paths.extract_if(|path| !is_unicode(path)).collect()
}
