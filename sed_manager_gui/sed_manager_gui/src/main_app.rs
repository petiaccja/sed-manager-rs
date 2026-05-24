use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

use sed_device::{Device, list_physical_drives, open_device};
use sed_manager_gui_slint as ui;
use sed_tper::Tper;
use slint::{ComponentHandle, Model, ModelExt, ModelRc, SharedString, ToSharedString, VecModel, spawn_local};

use crate::display_ui::DisplayUi;
use crate::toast::ToastQueue;

pub struct MainApp {
    ui: ui::MainWindow,
    notification_queue: Rc<ToastQueue>,
    devices: Rc<VecModel<ui::Device>>,
    services: Services,
}

impl MainApp {
    pub fn new(ui: ui::MainWindow, notification_queue: Rc<ToastQueue>) -> Rc<Self> {
        let view_model = Rc::from(Self {
            ui: ui.clone_strong(),
            notification_queue,
            devices: VecModel::from(Vec::new()).into(),
            services: Services::new(),
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
            let sorted = view_model.devices.clone().sort_by(|lhs, rhs| {
                (!lhs.security_commands, &lhs.name, &lhs.serial).cmp(&(!rhs.security_commands, &rhs.name, &rhs.serial))
            });
            ui.set_devices(ModelRc::from(Rc::from(sorted)));
        }

        view_model
    }

    fn scan(self: Rc<Self>) {
        let _ = spawn_local(async move {
            self.ui.set_scan_outcome(ui::Outcome::Pending);
            let new_paths = match list_physical_drives().await {
                Ok(paths) => paths,
                Err(err) => {
                    self.notification_queue.push(ui::Toast {
                        level: ui::ToastLevel::Error,
                        message: err.to_shared_string(),
                        title: "Drive scan failed".into(),
                    });
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
                        self.notification_queue.push(ui::Toast {
                            level: ui::ToastLevel::Warning,
                            message: format!(
                                "Non-unicode drive paths are not supported. The drive {} will be ignored.",
                                path.to_string_lossy()
                            )
                            .into(),
                            title: "Non-unicode drive path".into(),
                        });
                        false
                    }
                })
                .collect();

            self.devices.retain(|device| new_paths.contains(Path::new(device.path.as_str())));
            self.services.retain(|path, _| new_paths.contains(path));

            for path in new_paths {
                if !self.services.contains(&path) {
                    let path_str = path.to_string_lossy().into_owned().into();
                    self.services.insert(path.clone());
                    self.devices.push(ui::Device {
                        path: path_str,
                        status: ui::Status { outcome: ui::Outcome::Idle, ..Default::default() },
                        ..Default::default()
                    });
                    self.clone().open(path);
                }
            }
        });
    }

    fn close(self: Rc<Self>, path: SharedString) {
        self.devices.retain(|device| device.path != path);
        self.services.retain(|path_, _| path.as_str() != path_);
    }

    fn open(self: Rc<Self>, path: PathBuf) {
        let _ = spawn_local(async move {
            self.devices.update(
                |device| device.path == path.to_string_lossy(),
                |device| ui::Device {
                    status: ui::Status { outcome: ui::Outcome::Pending, ..Default::default() },
                    ..device
                },
            );

            let device = match open_device(&path).await {
                Ok(device) => device,
                Err(err) => {
                    self.devices.update(
                        |device| device.path == path.to_string_lossy(),
                        |device| ui::Device {
                            status: ui::Status { outcome: ui::Outcome::Error, message: err.to_shared_string() },
                            ..device
                        },
                    );
                    return;
                }
            };

            if device.is_security_supported() {
                self.clone().discover(path.clone());
            }

            self.devices.update(|device| device.path == path.to_string_lossy(), |_| device.display_ui());
            self.services.set_device(path, device);
        });
    }

    fn discover(self: Rc<Self>, path: PathBuf) {
        let _ = spawn_local(async move {
            let Some(device) = self.services.get_device(&path) else {
                return;
            };
            let discovery = match Tper::discover(&*device).await {
                Ok(discovery) => discovery.display_ui(),
                Err(err) => ui::Discovery {
                    status: ui::Status { message: err.to_shared_string(), outcome: ui::Outcome::Error },
                    ..Default::default()
                },
            };
            self.devices.update(
                |device| device.path == path.to_string_lossy(),
                |device| ui::Device { discovery: discovery, ..device },
            );
        });
    }
}

#[derive(Debug, Default)]
struct Service {
    device: Option<Arc<dyn Device>>,
    tper: Option<Arc<Tper>>,
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
