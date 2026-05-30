use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    ops::DerefMut,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

use sed_async_runtime::Runtime;
use sed_device::{Device, list_physical_drives, open_device};
use sed_manager::{Error, SidSession, Spec};
use sed_manager_gui_slint as ui;
use sed_packet::MaxBytes;
use sed_tper::Tper;
use sed_virtual_device::{VIRTUAL_DEVICE_PATH, VirtualDevice};
use slint::{
    ComponentHandle, EventLoopError, Model, ModelExt, ModelRc, SharedString, ToSharedString, VecModel, spawn_local,
};
use tracing::{Instrument, error, instrument};

use crate::display_ui::DisplayUi;
use crate::toast::ToastQueue;

pub struct App {
    ui: ui::MainWindow,
    toast_queue: Rc<ToastQueue>,
    devices: Rc<VecModel<ui::Device>>,
    services: Services,
    runtime: Rc<Runtime>,
}

impl App {
    pub fn new(ui: ui::MainWindow, notification_queue: Rc<ToastQueue>, runtime: Rc<Runtime>) -> Rc<Self> {
        let view_model = Rc::from(Self {
            ui: ui.clone_strong(),
            toast_queue: notification_queue,
            devices: VecModel::from(Vec::new()).into(),
            services: Services::new(),
            runtime,
        });

        {
            let view_model = view_model.clone();
            ui.on_scan(move || view_model.clone().scan());
        }
        {
            let view_model = view_model.clone();
            ui.on_close(move |path| view_model.clone().close(path));
        }
        {
            let view_model = view_model.clone();
            ui.on_take_owneship(move |path, password| {
                view_model.clone().take_ownership(path.to_string().into(), password)
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
        {
            let sorted = view_model.devices.clone().sort_by(|lhs, rhs| {
                fn key(identity: &ui::Identity) -> (bool, &SharedString, &SharedString) {
                    (!identity.security_commands, &identity.name, &identity.serial)
                }
                key(&lhs.identity).cmp(&key(&rhs.identity))
            });
            ui.set_devices(ModelRc::from(Rc::from(sorted)));
        }

        view_model
    }

    #[instrument(skip(self))]
    fn scan(self: Rc<Self>) {
        let future = async move {
            self.ui.set_scan_outcome(ui::Outcome::Pending);
            let new_paths = match list_physical_drives().await {
                Ok(paths) => paths,
                Err(err) => {
                    self.toast_queue.error("Drive scan failed".into(), err.to_string());
                    self.ui.set_scan_outcome(ui::Outcome::Error);
                    return;
                }
            };
            self.ui.set_scan_outcome(ui::Outcome::Success);

            // Discard paths that are not unicode. This is a must because the
            // paths as strings are used as a unique ID for the devices.
            let new_paths: HashSet<_> = new_paths
                .into_iter()
                .filter(|path| match path.to_str() {
                    Some(_) => true,
                    None => {
                        self.toast_queue.warning(
                            "Non-unicode drive path".into(),
                            format!(
                                "Non-unicode drive paths are not supported. The drive {} will be ignored.",
                                path.to_string_lossy()
                            ),
                        );
                        false
                    }
                })
                .collect();

            #[cfg(debug_assertions)]
            let new_paths = {
                let mut new_paths = new_paths;
                new_paths.insert(VIRTUAL_DEVICE_PATH.into());
                new_paths
            };

            self.devices.retain(|device| new_paths.contains(Path::new(device.identity.path.as_str())));
            self.services.retain(|path, _| new_paths.contains(path));

            for path in new_paths {
                if !self.services.contains(&path) {
                    let path_str = path.to_string_lossy().into_owned().into();
                    self.services.insert(path.clone());
                    self.devices.push(ui::Device {
                        identity: ui::Identity { path: path_str, ..Default::default() },
                        status: ui::Status { outcome: ui::Outcome::Idle, ..Default::default() },
                        ..Default::default()
                    });
                    self.clone().open(path);
                }
            }
        };
        spawn_local(future.in_current_span()).expect_in_event_loop();
    }

    #[instrument(skip(self))]
    fn close(self: Rc<Self>, path: SharedString) {
        self.devices.retain(|device| device.identity.path != path);
        self.services.retain(|path_, _| path.as_str() != path_);
    }

    #[instrument(skip(self))]
    fn open(self: Rc<Self>, path: PathBuf) {
        let future = async move {
            self.devices.update(
                |device| device.identity.path == path.to_string_lossy(),
                |device| ui::Device {
                    status: ui::Status { outcome: ui::Outcome::Pending, ..Default::default() },
                    ..device
                },
            );

            let device = if &path != VIRTUAL_DEVICE_PATH {
                match open_device(&path).await {
                    Ok(device) => device,
                    Err(err) => {
                        self.devices.update(
                            |device| device.identity.path == path.to_string_lossy(),
                            |device| ui::Device {
                                status: ui::Status { outcome: ui::Outcome::Error, message: err.to_shared_string() },
                                ..device
                            },
                        );
                        return;
                    }
                }
            } else {
                Box::new(VirtualDevice::new()) as _
            };

            if device.is_security_supported() {
                self.clone().discover(path.clone());
                self.clone().connect(path.clone());
            }

            self.devices.update(
                |device| device.identity.path == path.to_string_lossy(),
                |old| ui::Device {
                    status: ui::Status { outcome: ui::Outcome::Success, ..Default::default() },
                    identity: device.display_ui(),
                    ..old
                },
            );
            self.services.set_device(path, device);
        };
        spawn_local(future.in_current_span()).expect_in_event_loop();
    }

    #[instrument(skip(self))]
    fn discover(self: Rc<Self>, path: PathBuf) {
        let future = async move {
            let Some(device) = self.services.get_device(&path) else {
                return;
            };
            self.devices.update(
                |device| device.identity.path == path.to_string_lossy(),
                |device| ui::Device {
                    discovery: ui::Discovery {
                        status: ui::Status { outcome: ui::Outcome::Pending, ..Default::default() },
                        ..Default::default()
                    },
                    ..device
                },
            );
            let discovery = match Tper::discover(&*device).await {
                Ok(discovery) => discovery.display_ui(),
                Err(err) => ui::Discovery {
                    status: ui::Status { message: err.to_shared_string(), outcome: ui::Outcome::Error },
                    ..Default::default()
                },
            };
            self.devices.update(
                |device| device.identity.path == path.to_string_lossy(),
                |device| ui::Device { discovery: discovery, ..device },
            );
        };
        spawn_local(future.in_current_span()).expect_in_event_loop();
    }

    #[instrument(skip(self))]
    fn connect(self: Rc<Self>, path: PathBuf) {
        let future = async move {
            let Some(device) = self.services.get_device(&path) else {
                return;
            };
            let Ok(discovery) = Tper::discover(&*device).await else {
                return;
            };
            let Some(spec) = Spec::new(discovery) else {
                return;
            };
            let Some(ssc) = spec.default_ssc() else {
                return;
            };
            let Some(com_id) = ssc.static_com_ids_p1().next() else {
                return;
            };
            let com_id_ext = 0;
            let tper = Tper::connect(com_id, com_id_ext, device, Some(&self.runtime));
            self.services.set_tper(path.clone(), tper);
            self.query_stack_status(path, true);
        };
        spawn_local(future.in_current_span()).expect_in_event_loop();
    }

    #[instrument(skip(self))]
    fn query_stack_status(self: Rc<Self>, path: PathBuf, silent: bool) {
        let future = async move {
            let stack_status = if let Some(tper) = self.services.get_tper(&path) {
                let com_id_status = match tper.verify_com_id_valid(tper.com_id(), tper.com_id_ext()).await {
                    Ok(status) => status.to_string(),
                    Err(_) => "query failed".into(),
                };
                ui::StackStatus {
                    com_id: tper.com_id().into(),
                    com_id_ext: tper.com_id_ext().into(),
                    com_id_status: com_id_status.into(),
                    connected: true,
                }
            } else {
                ui::StackStatus { connected: false, ..Default::default() }
            };
            if !silent {
                self.toast_queue.info("Stack status updated".into(), "".into());
            }
            self.devices
                .update(|device| device.identity.path.as_str() == &path, |old| ui::Device { stack_status, ..old });
        };
        spawn_local(future.in_current_span()).expect_in_event_loop();
    }

    #[instrument(skip(self))]
    fn reset_stack(self: Rc<Self>, path: PathBuf) {
        let future = async move {
            if let Some(tper) = self.services.get_tper(&path) {
                match tper.stack_reset(tper.com_id(), tper.com_id_ext()).await {
                    Ok(_) => self.toast_queue.success("Stack has been reset".into(), "".into()),
                    Err(err) => self.toast_queue.error("Stack reset failed".into(), err.to_string()),
                };
            }
        };
        spawn_local(future.in_current_span()).expect_in_event_loop();
    }

    #[instrument(skip(self))]
    fn take_ownership(self: Rc<Self>, path: PathBuf, password: SharedString) {
        let future = async move {
            let Some(session) = self.services.get_session(&path) else {
                return;
            };
            let mut session = session.lock().await;
            let Some(sid_session) = self.clone().get_or_start_sid_session(&path, session.deref_mut()).await else {
                return;
            };

            let Some(password) = self.try_convert_password(password.as_str()) else {
                return;
            };

            match sid_session.take_owneship(password).await {
                Ok(_) => self.toast_queue.success("Taken ownership successfully".into(), "".into()),
                Err(err) => self.toast_queue.error("Taking ownership failed".into(), err.to_string()),
            };
        };
        spawn_local(future.in_current_span()).expect_in_event_loop();
    }

    #[instrument(skip(self))]
    fn revert_device(
        self: Rc<Self>,
        path: PathBuf,
        scope: ui::RevertScope,
        authority: ui::RevertAuthority,
        password: SharedString,
    ) {
        let future = async move {
            let Some(session) = self.services.get_session(&path) else {
                return;
            };
            let mut session = session.lock().await;
            let Some(sid_session) = self.clone().get_or_start_sid_session(&path, session.deref_mut()).await else {
                return;
            };

            let Some(password) = self.try_convert_password(password.as_str()) else {
                return;
            };

            let authority = match authority {
                ui::RevertAuthority::Sid => sid_session.spec().admin.authorities.sid,
                ui::RevertAuthority::Psid => sid_session.spec().admin.authorities.psid,
            };

            let result = match scope {
                ui::RevertScope::Locking => sid_session.revert_secondary_sp(password).await,
                ui::RevertScope::Everything => sid_session.revert_tper(authority, password).await,
            };

            match result {
                Ok(_) => self.toast_queue.success("Reverted device successfully".into(), "".into()),
                Err(err) => self.toast_queue.error("Reverting device failed".into(), err.to_string()),
            };
        };
        spawn_local(future.in_current_span()).expect_in_event_loop();
    }

    async fn get_or_start_sid_session<'s>(
        self: Rc<Self>,
        path: &Path,
        session: &'s mut Session,
    ) -> Option<&'s SidSession> {
        match session.start_sid_session_with(|| self.services.get_tper(&path)).await {
            Some(Ok(sid_session)) => Some(sid_session),
            Some(Err(err)) => {
                self.toast_queue.error("Could not open SID session".into(), err.to_string());
                None
            }
            None => {
                // The UI's flow should prevent us from ever reaching this path.
                error!("Bug: path should be unreachable from UI");
                self.toast_queue.error("Not connected".into(), "Not connected to the drive.".into());
                None
            }
        }
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
}

#[derive(Debug, Default)]
struct Service {
    device: Option<Arc<dyn Device>>,
    tper: Option<Arc<Tper>>,
    session: Arc<async_lock::Mutex<Session>>,
}

struct Services {
    inner: RefCell<HashMap<PathBuf, Service>>,
}

impl Services {
    pub fn new() -> Self {
        Self { inner: HashMap::new().into() }
    }

    pub fn contains(&self, path: &Path) -> bool {
        self.inner.borrow().contains_key(path)
    }

    pub fn retain<F>(&self, f: F)
    where
        F: FnMut(&PathBuf, &mut Service) -> bool,
    {
        self.inner.borrow_mut().retain(f);
    }

    pub fn insert(&self, path: PathBuf) {
        self.inner.borrow_mut().insert(path, Service::default());
    }

    pub fn set_device(&self, path: impl AsRef<Path>, device: Box<dyn Device>) {
        self.inner.borrow_mut().get_mut(path.as_ref()).map(|service| {
            service.device = Some(device.into());
        });
    }

    pub fn get_device(&self, path: impl AsRef<Path>) -> Option<Arc<dyn Device>> {
        self.inner.borrow().get(path.as_ref()).map(|service| service.device.clone()).flatten()
    }

    pub fn set_tper(&self, path: impl AsRef<Path>, tper: Tper) {
        self.inner.borrow_mut().get_mut(path.as_ref()).map(|service| {
            service.tper = Some(tper.into());
        });
    }

    pub fn get_tper(&self, path: impl AsRef<Path>) -> Option<Arc<Tper>> {
        self.inner.borrow().get(path.as_ref()).map(|service| service.tper.clone()).flatten()
    }

    pub fn get_session(&self, path: impl AsRef<Path>) -> Option<Arc<async_lock::Mutex<Session>>> {
        self.inner.borrow().get(path.as_ref()).map(|service| service.session.clone())
    }
}

#[derive(Debug, Default)]
enum Session {
    #[default]
    None,
    Sid(SidSession),
}

impl Session {
    pub async fn start_sid_session(&mut self, tper: Arc<Tper>) -> Result<&SidSession, Error> {
        match self {
            Self::None => {
                let sid_session = SidSession::on_primary_ssc(tper).await?;
                *self = Self::Sid(sid_session);
                let Self::Sid(sid_session) = self else { unreachable!() };
                Ok(sid_session)
            }
            Self::Sid(sid_session) => Ok(sid_session),
        }
    }

    pub async fn start_sid_session_with(
        &mut self,
        tper: impl FnOnce() -> Option<Arc<Tper>>,
    ) -> Option<Result<&SidSession, Error>> {
        match self {
            Self::None => Some(self.start_sid_session(tper()?).await),
            Self::Sid(sid_session) => Some(Ok(sid_session)),
        }
    }
}

pub trait VecModelExt<T> {
    fn retain<F>(&self, f: F)
    where
        F: FnMut(&T) -> bool;

    fn update<F, U>(&self, find: F, update: U)
    where
        F: FnMut(&T) -> bool,
        U: FnOnce(T) -> T;
}

impl<T> VecModelExt<T> for VecModel<T>
where
    T: Clone + 'static,
{
    fn retain<F>(&self, mut f: F)
    where
        F: FnMut(&T) -> bool,
    {
        let mut not_retained: Vec<_> =
            self.iter().enumerate().filter(|(_, item)| !f(item)).map(|(index, _)| index).collect();
        not_retained.sort();
        not_retained.reverse();
        for index in not_retained {
            self.remove(index);
        }
    }

    fn update<F, U>(&self, mut find: F, update: U)
    where
        F: FnMut(&T) -> bool,
        U: FnOnce(T) -> T,
    {
        self.iter().enumerate().find(|(_, value)| find(value)).map(|(index, value)| {
            self.set_row_data(index, update(value));
        });
    }
}

trait ExpectInEventLoop {
    type Output;

    fn expect_in_event_loop(self) -> Self::Output;
}

impl<T> ExpectInEventLoop for Result<T, EventLoopError> {
    type Output = T;
    fn expect_in_event_loop(self) -> Self::Output {
        self.expect("expected to be inside the event loop")
    }
}
