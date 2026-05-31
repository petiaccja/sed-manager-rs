use std::{
    collections::{HashMap, HashSet},
    ops::DerefMut,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

use async_lock::{Mutex, RwLock};
use futures::FutureExt;
use sed_async_runtime::Runtime;
use sed_device::{Device, list_physical_drives, open_device};
use sed_manager::{Error, SidSession, Spec};
use sed_manager_gui_slint::{self as ui};
use sed_packet::MaxBytes;
use sed_tper::Tper;
use sed_virtual_device::{VIRTUAL_DEVICE_PATH, VirtualDevice};
use slint::{ComponentHandle, EventLoopError, ModelExt as _, ModelRc, SharedString, ToSharedString, spawn_local};
use tracing::{Instrument, instrument};

use crate::{associative_model::AssociativeModel, display_ui::DisplayUi as _, ui_ext::DiscoveryExt};
use crate::{toast::ToastQueue, ui_ext::DeviceExt as _};

pub struct App {
    ui: ui::MainWindow,
    device_list: RwLock<DeviceList>,
    toast_queue: Rc<ToastQueue>,
    runtime: Rc<Runtime>,
}

impl App {
    pub fn new(ui: ui::MainWindow, notification_queue: Rc<ToastQueue>, runtime: Rc<Runtime>) -> Rc<Self> {
        let device_list = DeviceList::default();
        let device_list_ui = device_list.ui.inner();

        let view_model = Rc::from(Self {
            ui: ui.clone_strong(),
            toast_queue: notification_queue,
            device_list: device_list.into(),
            runtime,
        });

        // Callbacks
        {
            let view_model = view_model.clone();
            ui.on_scan(move || view_model.clone().scan(false));
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

    #[instrument(skip(self))]
    pub fn scan(self: Rc<Self>, silent: bool) {
        let app = self.clone();
        let future = async move {
            app.ui.set_scan_outcome(ui::Outcome::Pending);
            let mut new_paths: HashSet<_> = list_physical_drives().await?.into_iter().collect();

            // The paths must be losslessly converted to Slint string because
            // they are used as HashMap keys.
            retain_unicode(&mut new_paths, &app.toast_queue);

            // Insert virtual device in debug mode.
            #[cfg(debug_assertions)]
            new_paths.insert(VIRTUAL_DEVICE_PATH.into());

            let mut device_list = app.device_list.write().await;
            device_list.backend.retain(|path, _| new_paths.contains(path));
            device_list.ui.retain(|path| new_paths.contains(path));

            for path in new_paths {
                if !device_list.backend.contains_key(&path) {
                    let path_str = path.to_string_lossy().to_shared_string();
                    device_list.backend.insert(path.clone(), Default::default());
                    device_list.ui.insert(
                        path.clone(),
                        ui::Device {
                            identity: ui::Identity { path: path_str, ..Default::default() },
                            status: ui::Status { outcome: ui::Outcome::Idle, ..Default::default() },
                            ..Default::default()
                        },
                    );
                    app.clone().open(path);
                }
            }

            Ok::<(), Error>(())
        };

        let app = self.clone();
        let future = future.then(move |result| async move {
            app.ui.set_scan_outcome(ui::Outcome::Idle);
            match result {
                Ok(_) if !silent => app.toast_queue.success("Device list updated".into(), "".into()),
                Err(err) => app.toast_queue.success("Could not update device list".into(), err.to_string()),
                _ => (),
            }
        });

        spawn_local(future.in_current_span()).expect_in_event_loop();
    }

    #[instrument(skip(self))]
    fn close(self: Rc<Self>, path: SharedString) {
        let app = self.clone();

        let future = async move {
            self.ui.set_scan_outcome(ui::Outcome::Pending);
            let path = PathBuf::from(String::from(path));
            let mut device_list = self.device_list.write().await;
            device_list.ui.remove(&path);
            if let Some(backend) = device_list.backend.remove(&path) {
                let mut backend = backend.lock().await;
                backend.session.close().await;
            }
        }
        .then(|_| async move {
            app.ui.set_scan_outcome(ui::Outcome::Idle);
        });

        spawn_local(future.in_current_span()).expect_in_event_loop();
    }

    #[instrument(skip(self))]
    fn open(self: Rc<Self>, path: PathBuf) {
        let future = async move {
            let device_list = self.device_list.read().await;
            let Some(backend) = device_list.backend.get(&path) else {
                return;
            };

            device_list.ui.update(&path, |device| device.with_status(ui::Outcome::Pending, "".into()));

            let result = if &path != VIRTUAL_DEVICE_PATH {
                open_device(&path).await
            } else {
                Ok(Box::new(VirtualDevice::new()) as _)
            };

            match result {
                Ok(device) => {
                    let is_security_supported = device.is_security_supported();
                    let identity = device.display_ui();
                    let mut backend = backend.lock().await;
                    backend.device = Some(device.into());
                    if is_security_supported {
                        self.clone().discover(path.clone());
                        self.clone().connect(path.clone());
                    }
                    device_list.ui.update(&path, |device| {
                        device.with_status(ui::Outcome::Success, "".into()).with_identity(identity)
                    });
                }
                Err(err) => {
                    device_list.ui.update(&path, |device| device.with_status(ui::Outcome::Error, err.to_string()));
                }
            };
        };
        spawn_local(future.in_current_span()).expect_in_event_loop();
    }

    #[instrument(skip(self))]
    fn discover(self: Rc<Self>, path: PathBuf) {
        let future = async move {
            let device_list = self.device_list.read().await;
            let Some(backend) = device_list.backend.get(&path) else {
                return;
            };
            let mut backend = backend.lock().await;
            let Some(device) = &backend.device else {
                return;
            };

            device_list.ui.update(&path, |device| ui::Device {
                discovery: ui::Discovery {
                    status: ui::Status { outcome: ui::Outcome::Pending, ..Default::default() },
                    ..Default::default()
                },
                ..device
            });

            match Tper::discover(device.as_ref()).await {
                Ok(mut discovery) => {
                    Spec::sort(&mut discovery);
                    let display = discovery.display_ui();
                    backend.spec = Spec::try_from(discovery).ok();
                    device_list.ui.update(&path, |device| device.with_discovery(display));
                }
                Err(err) => {
                    device_list.ui.update(&path, |device| device.with_discovery(ui::Discovery::error(err.to_string())));
                }
            };
        };
        spawn_local(future.in_current_span()).expect_in_event_loop();
    }

    #[instrument(skip(self))]
    fn connect(self: Rc<Self>, path: PathBuf) {
        let future = async move {
            let self_ = self.clone();
            let device_list = self_.device_list.read().await;
            let Some(backend) = device_list.backend.get(&path) else {
                return;
            };
            let mut backend = backend.lock().await;
            let BackendDevice { device, spec, tper, .. } = backend.deref_mut();
            let (Some(device), Some(spec)) = (device, spec) else {
                return;
            };
            let Some(ssc) = spec.default_ssc() else {
                return;
            };
            let Some(com_id) = ssc.static_com_ids_p1().next() else {
                return;
            };
            let com_id_ext = 0;
            *tper = Some(Tper::connect(com_id, com_id_ext, device.clone(), Some(&self.runtime)).into());
            self.query_stack_status(path, true);
        };
        spawn_local(future.in_current_span()).expect_in_event_loop();
    }

    #[instrument(skip(self))]
    fn query_stack_status(self: Rc<Self>, path: PathBuf, silent: bool) {
        let future = async move {
            let device_list = self.device_list.read().await;
            let backend = device_list.backend.get(&path)?;
            let backend = backend.lock().await;
            let tper = backend.tper.as_ref()?;
            let result = tper.verify_com_id_valid(tper.com_id(), tper.com_id_ext()).await;

            let ui_status_base = ui::StackStatus {
                com_id: tper.com_id().into(),
                com_id_ext: tper.com_id_ext().into(),
                com_id_status: "".into(),
                connected: true,
            };

            match result {
                Ok(status) => {
                    let ui_status = ui::StackStatus { com_id_status: status.to_shared_string(), ..ui_status_base };
                    device_list.ui.update(&path, |device| device.with_stack_status(ui_status));
                    if !silent {
                        self.toast_queue.success("Stack status updated".into(), "".to_string());
                    }
                }
                Err(err) => {
                    let ui_status = ui::StackStatus { com_id_status: err.to_shared_string(), ..ui_status_base };
                    device_list.ui.update(&path, |device| device.with_stack_status(ui_status));
                    if !silent {
                        self.toast_queue.error("Could not update stack status".into(), err.to_string());
                    }
                }
            };

            Some(())
        };
        spawn_local(future.in_current_span()).expect_in_event_loop();
    }

    #[instrument(skip(self))]
    fn reset_stack(self: Rc<Self>, path: PathBuf) {
        let future = async move {
            let device_list = self.device_list.read().await;
            let backend = device_list.backend.get(&path)?;
            let backend = backend.lock().await;
            let tper = backend.tper.as_ref()?;

            match tper.stack_reset(tper.com_id(), tper.com_id_ext()).await {
                Ok(_) => self.toast_queue.success("Stack has been reset".into(), "".into()),
                Err(err) => self.toast_queue.error("Could not reset stack".into(), err.to_string()),
            };

            Some(())
        };
        spawn_local(future.in_current_span()).expect_in_event_loop();
    }

    #[instrument(skip(self))]
    fn take_ownership(self: Rc<Self>, path: PathBuf, password: SharedString) {
        let future = async move {
            let device_list = self.device_list.read().await;
            let backend = device_list.backend.get(&path)?;
            let mut backend = backend.lock().await;
            let sid_session = self.start_sid_session(backend.deref_mut()).await?;

            let password = self.try_convert_password(password.as_str())?;
            match sid_session.take_owneship(password).await {
                Ok(_) => self.toast_queue.success("Taken ownership".into(), "".into()),
                Err(err) => self.toast_queue.error("Could not take ownership".into(), err.to_string()),
            };

            Some(())
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
            let device_list = self.device_list.read().await;
            let backend = device_list.backend.get(&path)?;
            let mut backend = backend.lock().await;
            let sid_session = self.start_sid_session(backend.deref_mut()).await?;

            let password = self.try_convert_password(password.as_str())?;
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

            Some(())
        };
        spawn_local(future.in_current_span()).expect_in_event_loop();
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

    async fn start_sid_session<'a>(&self, backend: &'a mut BackendDevice) -> Option<&'a SidSession> {
        match backend.session.start_sid_session(backend.tper.clone()?).await {
            Ok(sid_session) => Some(sid_session),
            Err(err) => {
                self.toast_queue.error("Could not start SID session".into(), err.to_string());
                None
            }
        }
    }
}

fn retain_unicode(paths: &mut HashSet<PathBuf>, toast_queue: &ToastQueue) {
    fn is_unicode(path: &Path) -> bool {
        path.to_str().is_some()
    }

    for path in paths.iter() {
        if !is_unicode(path) {
            toast_queue.warning(
                "Ignoring drive".into(),
                format!("The drive '{}' has a non-unicode path and will be ignored.", path.to_string_lossy()),
            )
        }
    }

    paths.retain(|path| is_unicode(path));
}

#[derive(Default)]
struct DeviceList {
    ui: AssociativeModel<PathBuf, ui::Device>,
    backend: HashMap<PathBuf, Mutex<BackendDevice>>,
}

#[derive(Debug, Default)]
struct BackendDevice {
    device: Option<Arc<dyn Device>>,
    spec: Option<Spec>,
    tper: Option<Arc<Tper>>,
    session: Session,
}

#[derive(Debug, Default)]
enum Session {
    #[default]
    None,
    Sid(SidSession),
}

impl Session {
    pub async fn close(&mut self) {
        match core::mem::replace(self, Session::None) {
            Session::None => (),
            Session::Sid(_sid_session) => (), // Not a persistent session, just drop.
        }
    }

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
