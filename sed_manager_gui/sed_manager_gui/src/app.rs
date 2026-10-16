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
use sed_manager::{Device, Error, Geometry, Host, Spec};
use sed_manager_gui_slint as ui;
use sed_packet::{MaxBytes, com_id_request::ComIdState};
use sed_spec::{
    methods::MethodStatus,
    objects::{Authority, AuthorityRef, LockingRangeRef, SecurityProviderRef},
    preconfig::{core::shared::authority::ANYBODY, opal_2::locking},
};
use sed_tper::{Error as TperError, PropertiesChanged};
use slint::{
    CloseRequestResponse, ComponentHandle, Model, ModelExt as _, ModelRc, SharedString, ToSharedString,
    quit_event_loop, spawn_local,
};
use tracing::{error, instrument};

use crate::{
    algorithm::sector_lib,
    command::{Command, CommandError, ExpectInEventLoop},
    device_list::{DeviceEntry, DeviceList},
    diff_model::{self, DiffModel},
    session::Session,
    toast::ToastQueue,
    ui_conv::{CombinedProperties, ConversionError, IntoUi, IntoUiName, MbrDesc, TryFromUi},
    ui_ext::{DeviceExt as _, DiscoveryExt, StackStatusExt},
};

pub struct App {
    ui: ui::MainWindow,
    device_list: Rc<RwLock<DeviceList>>,
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
            device_list: Rc::new(RwLock::new(device_list)),
            host,
            runtime,
        });

        // Algorithm callbacks.
        ui.global::<ui::LockingRangeLib>().on_is_any_dirty(diff_model::is_any_dirty);
        ui.global::<ui::LockingRangeLib>().on_is_individual_dirty(diff_model::is_individual_dirty);
        ui.global::<ui::LockingRangeLib>().on_discard(diff_model::discard);
        ui.global::<ui::AuthorityLib>().on_is_any_dirty(diff_model::is_any_dirty);
        ui.global::<ui::AuthorityLib>().on_is_individual_dirty(diff_model::is_individual_dirty);
        ui.global::<ui::AuthorityLib>().on_discard(diff_model::discard);

        ui.global::<ui::SectorLib>().on_to_string(sector_lib::to_string);
        ui.global::<ui::SectorLib>().on_is_valid(sector_lib::is_valid);
        ui.global::<ui::SectorLib>().on_parse(sector_lib::parse);
        ui.global::<ui::SectorLib>().on_align(sector_lib::align);

        // Action callbacks.
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
            ui.on_commit_locking_ranges(move |path| view_model.clone().commit_locking_ranges(path.to_string().into()));
        }
        {
            let view_model = view_model.clone();
            ui.on_erase_locking_range(move |path, uid| {
                view_model.clone().erase_locking_range(path.to_string().into(), uid)
            });
        }
        {
            let view_model = view_model.clone();
            ui.on_commit_authorities(move |path| {
                view_model.clone().commit_locking_authorities(path.to_string().into())
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
        Command::new(self.runtime.clone(), self.device_list.clone(), self.toast_queue.clone())
    }

    #[instrument(skip(self, silent))]
    pub fn scan(self: Rc<Self>, silent: bool) {
        self.ui.set_scan_outcome(ui::Outcome::Pending);
        let host = self.host.clone();
        let self_ = self.clone();

        self.command()
            .success_message_if((!silent).then_some("Device list updated"))
            .error_message("Could not update device list")
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

                Ok((added, removed, non_unicode))
            })
            .on_success(move |device_list, (added, removed, non_unicode)| {
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
                        "Device ignored",
                        format!("Device paths must be valid unicode strings. The device {path} will be ignored"),
                    );
                }
            })
            .finally(move |_| self_.ui.set_scan_outcome(ui::Outcome::Idle))
            .run();
    }

    #[instrument(skip(self))]
    fn close_device(self: Rc<Self>, path: SharedString) {
        self.ui.set_scan_outcome(ui::Outcome::Pending);
        let path = PathBuf::from(String::from(path));

        self.command()
            .on_device_list(async move |device_list| {
                device_list.ui.remove(&path);
                if let Some(device) = device_list.backend.remove(&path) {
                    let mut device = device.write().await;
                    device.close().await;
                }
                Ok(())
            })
            .finally(move |_| self.ui.set_scan_outcome(ui::Outcome::Idle))
            .run();
    }

    #[instrument(skip(self))]
    fn open(self: Rc<Self>, path: PathBuf) {
        let path_ = path.clone();
        let host = self.host.clone();
        self.command()
            .on_device_entry(path.clone(), async move |device_entry: &mut DeviceEntry| {
                let device = host.open_device(path_).await?;
                let storage_device = device.storage_device();
                let capabilities = device.capabilities().ok();
                let properties_changed = device.properties_changed().ok();
                let geometry = device
                    .geometry()
                    .await
                    .unwrap_or(Geometry { logical_sector_size: 0, logical_sector_count: 1u64 << 62 });
                device_entry.device = Some(device);
                Ok((storage_device, capabilities, properties_changed, geometry))
            })
            .on_success(move |mut ui_device, (storage_device, capabilities, properties_changed, geometry)| {
                if storage_device.is_security_supported() {
                    self.clone().discover(path.clone());
                }
                if let (Some(capabilities), Some(properties_changed)) = (capabilities, properties_changed) {
                    let combined_properties = CombinedProperties { host: capabilities, device: None, connection: None };
                    let status = ui_device.stack_status.clone().with_protocol((&combined_properties).into_ui());
                    spawn_local(Self::listen_connection_changed(
                        Rc::downgrade(&self),
                        path.clone(),
                        properties_changed,
                    ))
                    .expect_in_event_loop();
                    self.clone().query_stack_status(path.clone(), true);
                    self.clone().list_security_providers(path.clone());
                    ui_device.stack_status = status;
                    ui_device.geometry = (&geometry).into_ui();
                }
                ui_device.identity = (&*storage_device).into_ui();
                ui_device
            })
            .on_error(move |mut ui_device, err| {
                ui_device.identity.status = ui::Status { outcome: ui::Outcome::Error, message: err.to_shared_string() };
                ui_device
            })
            .run();
    }

    #[instrument(skip(self))]
    fn discover(self: Rc<Self>, path: PathBuf) {
        self.command()
            .on_device(path.clone(), async move |device: &Device| {
                let mut discovery = device.discover().await?;
                Spec::sort(&mut discovery);
                Ok(discovery)
            })
            .on_success(move |ui_device, discovery| {
                let (ui_config, ui_discovery) = (&discovery).into_ui();
                ui_device.with_discovery(ui_discovery).with_config(ui_config)
            })
            .on_error(move |ui_device, err| ui_device.with_discovery(ui::Discovery::error(err.to_string())))
            .run();
    }

    #[instrument(skip(self, silent))]
    fn query_stack_status(self: Rc<Self>, path: PathBuf, silent: bool) {
        // Get the ComID and the ComIDExt if the SSC is available and we're connected.
        self.command()
            .on_device(path.clone(), async |device: &Device| Ok((device.com_id()?, device.com_id_ext()?)))
            .on_success(move |mut ui_device, (com_id, com_id_ext)| {
                ui_device.stack_status.com_id.connected = true;
                ui_device.stack_status.com_id.com_id = com_id.0.into();
                ui_device.stack_status.com_id.com_id_ext = com_id_ext.0.into();
                ui_device
            })
            .on_error(|mut ui_device, _err| {
                ui_device.stack_status.com_id.connected = false;
                ui_device.stack_status.com_id.com_id = 0;
                ui_device.stack_status.com_id.com_id_ext = 0;
                ui_device
            })
            .run();

        // Get the status of the current connection.
        self.command()
            .success_message_if((!silent).then_some("Stack status updated"))
            .error_message_if((!silent).then_some("Could not update stack status"))
            .on_device(path.clone(), async |device: &Device| device.query_com_id_state().await.map_err(Into::into))
            .on_success(move |mut ui_device, com_id_state| {
                ui_device.stack_status.com_id.receptive =
                    matches!(com_id_state, ComIdState::Issued | ComIdState::Associated);
                ui_device.stack_status.com_id.status = com_id_state.to_shared_string();
                ui_device
            })
            .on_error(|mut ui_device, _err| {
                ui_device.stack_status.com_id.receptive = false;
                ui_device.stack_status.com_id.status = "N/A".to_shared_string();
                ui_device
            })
            .run();
    }

    #[instrument(skip(self))]
    fn list_security_providers(self: Rc<Self>, path: PathBuf) {
        self.command()
            .on_device(path.clone(), async |device: &Device| {
                let discovery = device.discover().await?;
                Spec::try_from(discovery).map_err(|_| Error::NoSscAvailable.into())
            })
            .on_success(move |mut ui_device, spec| {
                let features = &spec.discovery.feature_descriptors;
                ui_device.admin_sp.uid = (&spec.admin.uid).into_ui();
                ui_device.admin_sp.name = (&spec.admin.uid).into_ui_name(features, Some(spec.admin.uid));
                if let Some(locking_sp) = spec.locking {
                    ui_device.locking_sp.uid = (&locking_sp.uid).into_ui();
                    ui_device.locking_sp.name = (&locking_sp.uid).into_ui_name(features, Some(spec.admin.uid))
                }
                ui_device
            })
            .run();
    }

    #[instrument(skip(self))]
    fn list_admin_authorities(self: Rc<Self>, path: PathBuf, silent: bool) {
        self.command()
            .success_message_if((!silent).then_some("Admin authorities updated"))
            .error_message("Failed to list admin authorities")
            .on_session(path.clone(), async |device: &Device, session: &mut Session| {
                let setup_session = session.start_setup_session(device).await?;
                let sp_ref = setup_session.spec().admin.uid;
                let authorities = setup_session.list_authorities(sp_ref).await?;
                Ok((authorities, Some(sp_ref)))
            })
            .on_success(move |mut ui_device, spec, (authorities, sp_ref)| {
                let (authorities, individual_authority_names, individual_authority_uids) =
                    display_authorities(spec, &authorities, sp_ref);

                ui_device.admin_sp.authorities = authorities.into();
                ui_device.admin_sp.individual_authority_names = Rc::from(individual_authority_names).into();
                ui_device.admin_sp.individual_authority_uids = Rc::from(individual_authority_uids).into();

                ui_device
            })
            .run();
    }

    #[instrument(skip(self))]
    fn list_locking_authorities(self: Rc<Self>, path: PathBuf, silent: bool) {
        self.command()
            .success_message_if((!silent).then_some("Locking authorities updated"))
            .error_message("Failed to list locking authorities")
            .on_session(path.clone(), async |device: &Device, session: &mut Session| {
                let setup_session = session.start_setup_session(device).await?;
                if let Some(locking_sp) = &setup_session.spec().locking {
                    let authorities = setup_session.list_authorities(locking_sp.uid).await?;
                    Ok((authorities, Some(locking_sp.uid)))
                } else {
                    Ok((vec![], None))
                }
            })
            .on_success(move |mut ui_device, spec, (authorities, sp_ref)| {
                let (authorities, individual_authority_names, individual_authority_uids) =
                    display_authorities(spec, &authorities, sp_ref);

                ui_device.locking_sp.authorities = authorities.into();
                ui_device.locking_sp.individual_authority_names = Rc::from(individual_authority_names).into();
                ui_device.locking_sp.individual_authority_uids = Rc::from(individual_authority_uids).into();

                ui_device
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
            self.toast_queue
                .error("Invalid authority reference", "The UID is not an authority reference. Please report this bug.");
            return;
        };

        self.command()
            .error_message("Could not log in")
            .on_session(path.clone(), async move |device: &Device, session: &mut Session| {
                let _ = session.start_locking_config_session(device, authority, Some(password)).await?;
                Ok(())
            })
            .on_success(move |ui_device, _, _| {
                self.clone().get_alignment(path.clone());
                self.clone().list_locking_authorities_ext(path.clone(), true);
                self.clone().list_locking_ranges(path.clone(), true);
                self.clone().get_mbr(path.clone(), true);
                ui_device
            })
            .run();
    }

    #[instrument(skip(self))]
    fn get_alignment(self: Rc<Self>, path: PathBuf) {
        self.command()
            .error_message("Could not get alignment")
            .on_session(path.clone(), async |_device: &Device, session: &mut Session| {
                let Session::LockingConfig(locking_config_session) = &*session else {
                    return Err(CommandError::Skipped);
                };
                locking_config_session.get_alignment().await.map_err(Into::into)
            })
            .on_success(move |mut ui_device, _, alignment| {
                ui_device.alignment = (&alignment).into_ui();
                ui_device
            })
            .run();
    }

    #[instrument(skip(self))]
    fn list_locking_authorities_ext(self: Rc<Self>, path: PathBuf, silent: bool) {
        self.command()
            .success_message_if((!silent).then_some("Locking authorities updated"))
            .error_message("Failed to list locking authorities")
            .on_session(path.clone(), async |_device: &Device, session: &mut Session| {
                let Session::LockingConfig(locking_config_session) = &*session else {
                    return Err(CommandError::Skipped);
                };
                locking_config_session.get_authorities().await.map_err(Into::into)
            })
            .on_success(move |mut ui_device, spec, authorities| {
                let sp_ref = spec.and_then(|spec| spec.locking.as_ref()).map(|sp| sp.uid);
                let (authorities, individual_authority_names, individual_authority_uids) =
                    display_authorities(spec, &authorities, sp_ref);

                ui_device.locking_sp.authorities = authorities.into();
                ui_device.locking_sp.individual_authority_names = Rc::from(individual_authority_names).into();
                ui_device.locking_sp.individual_authority_uids = Rc::from(individual_authority_uids).into();
                ui_device
            })
            .run();
    }

    #[instrument(skip(self))]
    fn list_locking_ranges(self: Rc<Self>, path: PathBuf, silent: bool) {
        self.command()
            .success_message_if((!silent).then_some("Locking ranges updated"))
            .error_message("Failed to update locking ranges")
            .on_session(path.clone(), async |_device: &Device, session: &mut Session| {
                let Session::LockingConfig(locking_config_session) = &*session else {
                    return Err(CommandError::Skipped);
                };
                locking_config_session.get_locking_ranges().await.map_err(Into::into)
            })
            .on_success(move |mut ui_device, spec, ranges| {
                let features = spec.map(|spec| spec.discovery.feature_descriptors.as_slice()).unwrap_or(&[]);
                let sp_ref = spec.and_then(|spec| spec.locking.as_ref()).map(|sp| sp.uid);
                let ranges: Vec<ui::LockingRange> =
                    ranges.iter().map(|range| range.into_ui_name(features, sp_ref)).collect();
                ui_device.locking_sp.locking_ranges = Rc::from(DiffModel::from(ranges)).into();
                ui_device
            })
            .run();
    }

    #[instrument(skip(self))]
    fn get_mbr(self: Rc<Self>, path: PathBuf, silent: bool) {
        self.command()
            .success_message_if((!silent).then_some("Shadow MBR status updated"))
            .error_message("Failed to update shadow MBR status")
            .on_session(path.clone(), async |device: &Device, session: &mut Session| {
                let Session::LockingConfig(locking_config_session) = &*session else {
                    return Err(CommandError::Skipped);
                };

                let geometry = device.geometry().await?;

                let size = match locking_config_session.get_mbr_size().await {
                    Ok(size) => size,
                    Err(Error::TperError(TperError::MethodCallFailed(MethodStatus::InvalidParameter))) => {
                        return Ok(MbrDesc { supported: false, size: None, control: None });
                    }
                    Err(err) => return Err(err.into()),
                };
                let control = locking_config_session.get_mbr_control().await?;
                Ok(MbrDesc { supported: true, size: Some(size / geometry.logical_sector_size), control: Some(control) })
            })
            .on_success(move |mut ui_device, _spec, mbr| {
                ui_device.locking_sp.mbr = (&mbr).into_ui();
                ui_device
            })
            .run();
    }

    #[instrument(skip(self))]
    fn commit_locking_ranges(self: Rc<Self>, path: PathBuf) {
        slint::spawn_local(async move {
            let device_list = self.device_list.read().await;
            if let Some(ui_device) = device_list.ui.get(&path) {
                // Read the locking ranges from the UI and convert it to backend format.
                let (committed, desired) = match convert_diff_model(ui_device.locking_sp.locking_ranges.clone()) {
                    Ok(diff_locking_ranges) => diff_locking_ranges,
                    Err(err) => {
                        error!(error = %err, locking_ranges = ?ui_device.locking_sp.locking_ranges);
                        self.toast_queue.error(
                            "Could not write locking ranges",
                            format!("{err}. This is a bug, please report it."),
                        );
                        return;
                    }
                };

                // Initiate the actual command.
                let self_s = self.clone();
                let path_s = path.clone();
                let self_e = self.clone();
                let path_e = path.clone();
                self.command()
                    .success_message("Locking ranges written to device")
                    .error_message("Could not write locking ranges")
                    .on_session(path, async move |device: &Device, session: &mut Session| {
                        let Session::LockingConfig(locking_session) = session else {
                            return Err(CommandError::Skipped);
                        };
                        let geometry = device.geometry().await?;
                        locking_session
                            .set_locking_ranges(committed.as_deref(), &desired, geometry.logical_sector_count)
                            .await?;
                        Ok(())
                    })
                    .on_success(move |ui_device, _spec, _output| {
                        self_s.list_locking_ranges(path_s, true);
                        ui_device
                    })
                    .on_error(move |ui_device, _spec, err| {
                        match err {
                            Error::LockingRangeUpdateRejected(_) => (),
                            _ => self_e.list_locking_ranges(path_e, true),
                        }
                        ui_device
                    })
                    .run();
            }
        })
        .expect_in_event_loop();
    }

    #[instrument(skip(self))]
    fn erase_locking_range(self: Rc<Self>, path: PathBuf, locking_range: ui::Uid) {
        let Ok(locking_range) = LockingRangeRef::try_from_ui(&locking_range) else {
            error!(locking_range_ref = &locking_range.value, error = "invalid locking range reference");
            self.toast_queue
                .error("Invalid authority reference", "The UID is not an authority reference. Please report this bug.");
            return;
        };

        self.command()
            .success_message("Locking range erased")
            .error_message("Failed to erase locking range")
            .on_session(path, async move |_: &Device, session: &mut Session| {
                let Session::LockingConfig(locking_session) = session else {
                    return Err(CommandError::Skipped);
                };
                locking_session.erase_locking_range(locking_range).await?;
                Ok(())
            })
            .run();
    }

    #[instrument(skip(self))]
    fn commit_locking_authorities(self: Rc<Self>, path: PathBuf) {
        slint::spawn_local(async move {
            let device_list = self.device_list.read().await;
            if let Some(ui_device) = device_list.ui.get(&path) {
                // Read the authorities from the UI and convert it to backend format.
                let (committed, desired) = match convert_diff_model(ui_device.locking_sp.authorities.clone()) {
                    Ok(diff_authorities) => diff_authorities,
                    Err(err) => {
                        error!(error = %err, authorities = ?ui_device.locking_sp.authorities);
                        self.toast_queue
                            .error("Could not write authorities", format!("{err}. This is a bug, please report it."));
                        return;
                    }
                };

                // Initiate the actual command.
                let self_s = self.clone();
                let path_s = path.clone();
                let self_e = self.clone();
                let path_e = path.clone();
                self.command()
                    .success_message("Authorities written to device")
                    .error_message("Could not write authorities")
                    .on_session(path, async move |_device: &Device, session: &mut Session| {
                        let Session::LockingConfig(locking_session) = session else {
                            return Err(CommandError::Skipped);
                        };
                        locking_session.set_authorities(committed.as_deref(), &desired).await?;
                        Ok(())
                    })
                    .on_success(move |ui_device, _spec, _output| {
                        self_s.list_locking_authorities_ext(path_s, true);
                        ui_device
                    })
                    .on_error(move |ui_device, _spec, err| {
                        match err {
                            Error::AuthorityUpdateRejected(_) => (),
                            _ => self_e.list_locking_authorities_ext(path_e, true),
                        }
                        ui_device
                    })
                    .run();
            }
        })
        .expect_in_event_loop();
    }

    #[instrument(skip(self))]
    fn logout(self: Rc<Self>, path: PathBuf) {
        self.command()
            .on_session(path.clone(), async move |device: &Device, session: &mut Session| {
                if matches!(*session, Session::LockingConfig(_))
                    && let Err(close_err) = session.close().await
                {
                    device.stack_reset().await?;
                    Err(close_err.into())
                } else {
                    Ok(())
                }
            })
            .run();
    }

    #[instrument(skip(self))]
    fn reset_stack(self: Rc<Self>, path: PathBuf) {
        self.command()
            .success_message("Stack has been reset")
            .error_message("Could not reset stack")
            .on_device(path.clone(), async |device: &Device| device.stack_reset().await.map_err(Into::into))
            .run();
    }

    #[instrument(skip(self, password))]
    fn take_ownership(self: Rc<Self>, path: PathBuf, password: SharedString) {
        let Some(password) = self.try_convert_password(password.as_str()) else {
            return;
        };

        self.command()
            .success_message("Taken ownership")
            .error_message("Could not take ownership")
            .on_session(path.clone(), async move |device: &Device, session: &mut Session| {
                let sid_session = session.start_setup_session(device).await?;
                sid_session.take_owneship(password).await?;
                Ok(())
            })
            .on_success(move |ui_device, _, _| {
                self.clone().discover(path.clone());
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
            .success_message("Locking activated")
            .error_message("Could not activate locking")
            .on_session(path.clone(), async move |device: &Device, session: &mut Session| {
                let sid_session = session.start_setup_session(device).await?;
                sid_session.activate_secondary_sp(password).await?;
                Ok(())
            })
            .on_success(move |ui_device, _, _| {
                self.clone().discover(path.clone());
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
            error!(sp_ref = &sp.value, error = "invalid SP reference");
            self.toast_queue
                .error("Invalid SP reference", "The UID is not an SP reference. Please report this bug.");
            return;
        };
        let Ok(authority) = AuthorityRef::try_from_ui(&authority) else {
            error!(authority_ref = &authority.value, error = "invalid authority reference");
            self.toast_queue
                .error("Invalid authority reference", "The UID is not an authority reference. Please report this bug.");
            return;
        };

        self.command()
            .success_message("Password changed")
            .error_message("Could not change password")
            .on_session(path.clone(), async move |device: &Device, session: &mut Session| {
                let sid_session = session.start_setup_session(device).await?;
                sid_session.change_password(sp, authority, current_password, new_password).await?;
                Ok(())
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
            .success_message("Reverted device successfully")
            .error_message("Reverting device failed")
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
                .map_err(Into::into)
            })
            .on_success(move |ui_device, _, _| {
                self.clone().discover(path.clone());

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
                Ok(())
            })
            .finally(move |_| {
                // Once all device are closed we can quit the event loop.
                let _ = quit_event_loop();
            })
            .run();
    }

    fn try_convert_password(&self, password: &str) -> Option<MaxBytes<32>> {
        let byte_password: MaxBytes<32> = password.as_bytes().into();
        if byte_password.len() == password.len() {
            Some(byte_password)
        } else {
            self.toast_queue
                .error("Password too long", "The password cannot be longer than 32 bytes (32 Latin characters)");
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
                        .on_device(path.clone(), async |device: &Device| device.capabilities().map_err(Into::into))
                        .on_success(move |ui_device, host_capabilities| {
                            let combined = CombinedProperties {
                                host: host_capabilities,
                                device: Some(value.remote_properties),
                                connection: Some(value.connection_properties),
                            };
                            let status = ui_device.stack_status.clone().with_protocol((&combined).into_ui());
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

fn display_authorities(
    spec: Option<&Spec>,
    authorities: &[Authority],
    sp_ref: Option<SecurityProviderRef>,
) -> (
    Rc<DiffModel<ui::Authority>>,
    impl Model<Data = SharedString> + use<>,
    impl Model<Data = ui::Uid> + use<>,
) {
    // Convert the authority structures to their UI counterparts.
    let features = spec.map(|spec| spec.discovery.feature_descriptors.as_slice()).unwrap_or(&[]);
    let authorities: Vec<ui::Authority> = authorities.iter().map(|auth| auth.into_ui_name(features, sp_ref)).collect();
    let authorities = Rc::from(DiffModel::from(authorities));

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

fn convert_diff_model<UiObject, BackendObject>(
    locking_range: ModelRc<UiObject>,
) -> Result<(Option<Vec<BackendObject>>, Vec<BackendObject>), ConversionError>
where
    UiObject: 'static,
    BackendObject: for<'a> TryFromUi<&'a UiObject, Error = ConversionError>,
{
    let committed = locking_range
        .as_any()
        .downcast_ref::<DiffModel<UiObject>>()
        .map(|diff_model| diff_model.committed());
    let committed = committed
        .map(|committed| committed.iter().map(BackendObject::try_from_ui).collect::<Result<Vec<_>, _>>())
        .transpose();
    let desired = locking_range.iter().map(|range| BackendObject::try_from_ui(&range)).collect::<Result<Vec<_>, _>>();

    Ok((committed?, desired?))
}
