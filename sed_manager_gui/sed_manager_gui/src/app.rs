use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    rc::{Rc, Weak},
    sync::Arc,
};

use async_lock::RwLock;
use sed_async::Runtime;
use sed_device::{list_physical_drives, open_device};
use sed_manager::{Error, Spec};
use sed_manager_gui_slint as ui;
use sed_packet::{MaxBytes, com_id::ComIdState};
use sed_tper::{PropertiesChanged, Tper};
use sed_virtual_device::{VIRTUAL_DEVICE_PATH, VirtualDevice};
use slint::{ComponentHandle, ModelExt as _, ModelRc, SharedString, ToSharedString, VecModel, spawn_local};
use tracing::instrument;

use crate::{
    command::{Command, ExpectInEventLoop},
    device_list::DeviceList,
    display_ui::{CombinedProperties, DisplayUi},
    toast::ToastQueue,
    ui_ext::{DeviceExt as _, DiscoveryExt, StackStatusExt},
};

pub struct App {
    ui: ui::MainWindow,
    device_list: Arc<RwLock<DeviceList>>,
    toast_queue: Rc<ToastQueue>,
    runtime: Arc<Runtime>,
}

impl App {
    pub fn new(ui: ui::MainWindow, notification_queue: Rc<ToastQueue>, runtime: Arc<Runtime>) -> Rc<Self> {
        let device_list = DeviceList::default();
        let device_list_ui = device_list.ui.inner();

        let view_model = Rc::from(Self {
            ui: ui.clone_strong(),
            toast_queue: notification_queue,
            device_list: Arc::new(RwLock::new(device_list)),
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

        self.command()
            .on_device_list(async |device_list| {
                let mut new_paths: HashSet<_> = list_physical_drives().await?.into_iter().collect();

                // The paths must be losslessly converted to Slint string because
                // they are used as HashMap keys.
                let non_unicode = retain_unicode(&mut new_paths);

                // Insert virtual device in debug mode.
                #[cfg(debug_assertions)]
                new_paths.insert(VIRTUAL_DEVICE_PATH.into());

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
                    let session = device.read().await.session.clone();
                    session.lock().await.close().await;
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
        let device_path = path.clone();
        self.command()
            .on_device(path.clone(), async move |mut device| {
                let result = if device_path.as_path() != VIRTUAL_DEVICE_PATH {
                    open_device(&device_path).await.map(|dev| Arc::<dyn sed_device::Device>::from(dev))
                } else {
                    Ok(Arc::new(VirtualDevice::new()) as _)
                };

                if let Ok(dev) = &result {
                    device.interface = Some(dev.clone());
                }
                result
            })
            .display(move |ui_device, result| match result {
                Ok(dev) => {
                    if dev.is_security_supported() {
                        app.clone().discover(path.clone());
                        app.clone().connect(path.clone());
                    }
                    ui_device.with_identity(dev.display_ui())
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
            .on_device(path.clone(), async move |mut device| {
                let Some(sed_device) = device.interface.as_ref() else {
                    return None;
                };
                match Tper::discover(sed_device.as_ref()).await {
                    Ok(mut discovery) => {
                        Spec::sort(&mut discovery);
                        device.specification = Spec::try_from(discovery.clone()).ok();
                        Some(Ok(discovery))
                    }
                    Err(err) => Some(Err(err)),
                }
            })
            .display(move |ui_device, result| match result {
                Some(Ok(discovery)) => {
                    let (ui_config, ui_discovery) = discovery.display_ui();
                    ui_device.with_discovery(ui_discovery).with_config(ui_config)
                }
                Some(Err(err)) => ui_device.with_discovery(ui::Discovery::error(err.to_string())),
                None => ui_device,
            })
            .run();
    }

    #[instrument(skip(self))]
    fn connect(self: Rc<Self>, path: PathBuf) {
        let runtime = self.runtime.clone();
        self.command()
            .on_device(path.clone(), async move |mut device| {
                let sed_device = device.interface.clone()?;
                let com_id = {
                    let spec = device.specification.as_ref()?;
                    let ssc = spec.default_ssc()?;
                    ssc.static_com_ids_p1().next()?
                };
                let com_id_ext = 0;
                let new_tper = Arc::new(Tper::connect(com_id, com_id_ext, sed_device, Some(runtime.as_ref())));
                let capabilities = new_tper.capabilities();
                let connection_changed = new_tper.properties_changed();
                device.tper = Some(new_tper);
                Some((capabilities, connection_changed))
            })
            .display(move |ui_device, result| {
                let Some((capabilities, connection_changed)) = result else {
                    return ui_device;
                };
                let combined_properties = CombinedProperties { host: capabilities, device: None, connection: None };
                let status = ui_device.stack_status.clone().with_protocol(combined_properties.display_ui());
                spawn_local(Self::listen_connection_changed(Rc::downgrade(&self), path.clone(), connection_changed))
                    .expect_in_event_loop();
                self.clone().query_stack_status(path.clone(), true);
                self.clone().list_security_providers(path.clone());
                self.clone().list_admin_authorities(path);
                ui_device.with_stack_status(status)
            })
            .run();
    }

    #[instrument(skip(self, silent))]
    fn query_stack_status(self: Rc<Self>, path: PathBuf, silent: bool) {
        self.command()
            .on_tper(path.clone(), async |tper| {
                let result = tper.verify_com_id_valid(tper.com_id(), tper.com_id_ext()).await;
                (tper.com_id(), tper.com_id_ext(), result)
            })
            .display(move |ui_device, (com_id, com_id_ext, result)| {
                let ui_status_base =
                    ui::ComIdStatus { com_id: com_id.into(), com_id_ext: com_id_ext.into(), ..Default::default() };
                let ui_status = match result {
                    Ok(state) => {
                        let good = [ComIdState::Issued, ComIdState::Associated].contains(&state);
                        if !silent {
                            self.toast_queue.success("Stack status updated".into(), "".to_string());
                        }
                        ui::ComIdStatus { status: state.to_shared_string(), good, ..ui_status_base }
                    }
                    Err(err) => {
                        if !silent {
                            self.toast_queue.error("Could not update stack status".into(), err.to_string());
                        }
                        ui::ComIdStatus { status: err.to_shared_string(), ..ui_status_base }
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
            .on_tper(path.clone(), async |tper| {
                Spec::try_from(tper.discover_current().await?).map_err(|_| Error::NoSscAvailable)
            })
            .display(move |mut ui_device, spec| match spec {
                Ok(spec) => {
                    ui_device.admin_sp.uid = spec.admin.uid.display_ui();
                    ui_device.admin_sp.name = spec.admin.uid.to_shared_string();
                    if let Some(locking_sp) = spec.locking {
                        ui_device.locking_sp.uid = locking_sp.uid.display_ui();
                        ui_device.locking_sp.name = locking_sp.uid.to_shared_string();
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
    fn list_admin_authorities(self: Rc<Self>, path: PathBuf) {
        self.command()
            .on_session(path.clone(), async |tper, mut session| {
                let setup_session = session.start_setup_session(tper).await?;
                let admin_sp = setup_session.spec().admin.uid;
                setup_session.list_authorities(admin_sp).await
            })
            .display(move |mut ui_device, authorities| match authorities {
                Ok(authorities) => {
                    let users: Vec<_> = authorities.iter().map(DisplayUi::display_ui).collect();
                    let users = Rc::from(VecModel::from(users));
                    let individual_users = Rc::from(users.clone().filter(|user| !user.is_class && user.enabled));
                    let individual_users_names = individual_users.clone().map(|user| user.name);
                    let individual_users_uids = individual_users.clone().map(|user| user.uid);
                    ui_device.admin_sp.users = users.into();
                    ui_device.admin_sp.individual_user_names = Rc::from(individual_users_names).into();
                    ui_device.admin_sp.individual_user_uids = Rc::from(individual_users_uids).into();
                    ui_device
                }
                Err(err) => {
                    self.toast_queue.error("Can not list users".into(), err.to_string());
                    ui_device
                }
            })
            .run();
    }

    #[instrument(skip(self))]
    fn reset_stack(self: Rc<Self>, path: PathBuf) {
        let app = self.clone();
        self.command()
            .on_tper(path.clone(), async |tper| tper.stack_reset(tper.com_id(), tper.com_id_ext()).await)
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
            .on_session(path.clone(), async move |tper, mut session| {
                let sid_session = session.start_setup_session(tper).await?;
                sid_session.take_owneship(password).await
            })
            .display(move |ui_device, result| {
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
            .on_session(path.clone(), async move |tper, mut session| {
                let sid_session = session.start_setup_session(tper).await?;
                sid_session.activate_secondary_sp(password).await
            })
            .display(move |ui_device, result| {
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
            .on_session(path.clone(), async move |tper, mut session| {
                let sid_session = session.start_setup_session(tper).await?;
                let authority = match authority {
                    ui::RevertAuthority::Sid => sid_session.spec().admin.authorities.sid,
                    ui::RevertAuthority::Psid => sid_session.spec().admin.authorities.psid,
                };
                match scope {
                    ui::RevertScope::Locking => sid_session.revert_secondary_sp(password).await,
                    ui::RevertScope::Everything => sid_session.revert_tper(authority, password).await,
                }
            })
            .display(move |ui_device, result| {
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
                        .on_tper(path.clone(), async |tper| tper.capabilities())
                        .display(move |ui_device, host| {
                            let combined = CombinedProperties {
                                host,
                                device: Some(value.remote_properties),
                                connection: Some(value.connection_properties),
                            };
                            let status = ui_device.stack_status.clone().with_protocol(combined.display_ui());
                            ui_device.with_stack_status(status)
                        })
                        .run();
                }
                Err(async_broadcast::RecvError::Overflowed(_)) => (),
                Err(async_broadcast::RecvError::Closed) => break,
            }
        }
    }
}

fn retain_unicode(paths: &mut HashSet<PathBuf>) -> Vec<PathBuf> {
    fn is_unicode(path: &Path) -> bool {
        path.to_str().is_some()
    }

    paths.extract_if(|path| !is_unicode(path)).collect()
}
