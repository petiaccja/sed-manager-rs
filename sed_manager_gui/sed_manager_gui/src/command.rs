use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::sync::Arc;

use async_lock::RwLock;
use sed_async::{PolyRuntime, Runtime as _};
use sed_manager::Spec;
use sed_manager_gui_slint as ui;
use sed_tper::Tper;
use slint::EventLoopError;
use tracing::Instrument;

use crate::device_list::{Device, DeviceList};
use crate::session::Session;
use crate::ui_ext::{CommandStatusExt, DeviceExt};

//------------------------------------------------------------------------------
// Command initial
//------------------------------------------------------------------------------

pub struct Command {
    runtime: Arc<PolyRuntime>,
    device_list: Arc<RwLock<DeviceList>>,
}

impl Command {
    pub fn new(runtime: Arc<PolyRuntime>, device_list: Arc<RwLock<DeviceList>>) -> Self {
        Self { runtime, device_list }
    }

    pub fn on_device_list<RunFn, Output>(self, run_fn: RunFn) -> CommandOnDeviceList<RunFn, Output>
    where
        RunFn: for<'a> AsyncFnOnce(&'a mut DeviceList) -> Output + 'static,
    {
        let Self { device_list, .. } = self;
        CommandOnDeviceList { device_list, run_fn }
    }

    pub fn on_device<RunFn, RunFnFut, Output>(
        self,
        device: PathBuf,
        run_fn: RunFn,
    ) -> CommandOnDevice<RunFn, RunFnFut, Output>
    where
        RunFn: FnOnce(Mut<Device>) -> RunFnFut + Send + 'static,
        RunFnFut: Future<Output = Output> + Send,
        Output: Send + 'static,
    {
        let Self { runtime, device_list } = self;
        CommandOnDevice { runtime, device_list, device_id: device, run_fn }
    }

    pub fn on_tper<RunFn, RunFnFut, Output>(
        self,
        device_id: PathBuf,
        run_fn: RunFn,
    ) -> CommandOnTper<RunFn, RunFnFut, Output>
    where
        RunFn: FnOnce(Arc<Tper>) -> RunFnFut + Send + 'static,
        RunFnFut: Future<Output = Output> + Send,
        Output: Send + 'static,
    {
        let Self { runtime, device_list } = self;
        CommandOnTper { runtime, device_list, device_id, run_fn }
    }

    pub fn on_session<RunFn, RunFnFut, Output>(
        self,
        device_id: PathBuf,
        run_fn: RunFn,
    ) -> CommandOnSession<RunFn, RunFnFut, Output>
    where
        RunFn: FnOnce(Arc<Tper>, Mut<Session>) -> RunFnFut + Send + 'static,
        RunFnFut: Future<Output = Output> + Send,
        Output: Send + 'static,
    {
        let Self { runtime, device_list } = self;
        CommandOnSession { runtime, device_list, device_id, run_fn }
    }
}

//------------------------------------------------------------------------------
// Command on device list
//------------------------------------------------------------------------------

pub struct CommandOnDeviceList<RunFn, Output>
where
    RunFn: for<'a> AsyncFnOnce(&'a mut DeviceList) -> Output + 'static,
{
    device_list: Arc<RwLock<DeviceList>>,
    run_fn: RunFn,
}

impl<RunFn, Output> CommandOnDeviceList<RunFn, Output>
where
    RunFn: for<'a> AsyncFnOnce(&'a mut DeviceList) -> Output + 'static,
{
    pub fn display<UpdateFn>(self, update_fn: UpdateFn) -> UpdateOnDeviceList<RunFn, Output, UpdateFn>
    where
        UpdateFn: for<'a> FnOnce(&'a mut DeviceList, Output) + 'static,
    {
        let Self { device_list, run_fn, .. } = self;
        UpdateOnDeviceList { device_list, run_fn, update_fn }
    }
}

pub struct UpdateOnDeviceList<RunFn, Output, UpdateFn>
where
    RunFn: for<'a> AsyncFnOnce(&'a mut DeviceList) -> Output + 'static,
    UpdateFn: for<'a> FnOnce(&'a mut DeviceList, Output) + 'static,
{
    device_list: Arc<RwLock<DeviceList>>,
    run_fn: RunFn,
    update_fn: UpdateFn,
}

impl<RunFn, Output, UpdateFn> UpdateOnDeviceList<RunFn, Output, UpdateFn>
where
    RunFn: for<'a> AsyncFnOnce(&'a mut DeviceList) -> Output + 'static,
    UpdateFn: for<'a> FnOnce(&'a mut DeviceList, Output) + 'static,
{
    pub fn run(self) {
        let Self { device_list, run_fn, update_fn } = self;
        let _ = slint::spawn_local(
            async move {
                let mut device_list = device_list.write().await;
                let output = run_fn(device_list.deref_mut()).await;
                update_fn(device_list.deref_mut(), output);
            }
            .in_current_span(),
        );
    }
}

//------------------------------------------------------------------------------
// Command on device
//------------------------------------------------------------------------------

pub struct CommandOnDevice<RunFn, RunFnFut, Output>
where
    RunFn: FnOnce(Mut<Device>) -> RunFnFut + Send + 'static,
    RunFnFut: Future<Output = Output> + Send,
    Output: Send + 'static,
{
    runtime: Arc<PolyRuntime>,
    device_list: Arc<RwLock<DeviceList>>,
    device_id: PathBuf,
    run_fn: RunFn,
}

impl<RunFn, RunFnFut, Output> CommandOnDevice<RunFn, RunFnFut, Output>
where
    RunFn: FnOnce(Mut<Device>) -> RunFnFut + Send + 'static,
    RunFnFut: Future<Output = Output> + Send,
    Output: Send + 'static,
{
    pub fn display<UpdateFn>(self, update_fn: UpdateFn) -> UpdateOnDevice<RunFn, RunFnFut, Output, UpdateFn>
    where
        UpdateFn: FnOnce(ui::Device, Output) -> ui::Device + 'static,
    {
        let Self { runtime, device_list, device_id, run_fn } = self;
        UpdateOnDevice { runtime, device_list, device_id, run_fn, update_fn }
    }
}

pub struct UpdateOnDevice<RunFn, RunFnFut, Output, UpdateFn>
where
    RunFn: FnOnce(Mut<Device>) -> RunFnFut + Send + 'static,
    RunFnFut: Future<Output = Output> + Send,
    Output: Send + 'static,
    UpdateFn: FnOnce(ui::Device, Output) -> ui::Device + 'static,
{
    runtime: Arc<PolyRuntime>,
    device_list: Arc<RwLock<DeviceList>>,
    device_id: PathBuf,
    run_fn: RunFn,
    update_fn: UpdateFn,
}

impl<RunFn, RunFnFut, Output, UpdateFn> UpdateOnDevice<RunFn, RunFnFut, Output, UpdateFn>
where
    RunFn: FnOnce(Mut<Device>) -> RunFnFut + Send + 'static,
    RunFnFut: Future<Output = Output> + Send,
    Output: Send + 'static,
    UpdateFn: FnOnce(ui::Device, Output) -> ui::Device + 'static,
{
    pub fn run(self) {
        let Self { runtime, device_list, device_id, run_fn, update_fn } = self;
        slint::spawn_local(
            async move {
                let state = device_list.read().await;
                let Some(device) = state.backend.get(&device_id) else { return };
                let device = device.write_arc().await;
                let output =
                    runtime.spawn(async move { run_fn(Mut::RwLock(device)).await }.in_current_span()).await.unwrap();
                state.ui.update(&device_id, move |value| update_fn(value, output));
            }
            .in_current_span(),
        )
        .expect_in_event_loop();
    }
}

//------------------------------------------------------------------------------
// Command on Tper
//------------------------------------------------------------------------------

pub struct CommandOnTper<RunFn, RunFnFut, Output>
where
    RunFn: FnOnce(Arc<Tper>) -> RunFnFut + Send + 'static,
    RunFnFut: Future<Output = Output> + Send,
    Output: Send + 'static,
{
    runtime: Arc<PolyRuntime>,
    device_list: Arc<RwLock<DeviceList>>,
    device_id: PathBuf,
    run_fn: RunFn,
}

impl<RunFn, RunFnFut, Output> CommandOnTper<RunFn, RunFnFut, Output>
where
    RunFn: FnOnce(Arc<Tper>) -> RunFnFut + Send + 'static,
    RunFnFut: Future<Output = Output> + Send,
    Output: Send + 'static,
{
    pub fn display<UpdateFn>(self, update_fn: UpdateFn) -> UpdateOnTper<RunFn, RunFnFut, Output, UpdateFn>
    where
        UpdateFn: FnOnce(ui::Device, Output) -> ui::Device + 'static,
    {
        let Self { runtime, device_list, device_id, run_fn } = self;
        UpdateOnTper { runtime, device_list, device_id, run_fn, update_fn }
    }
}

pub struct UpdateOnTper<RunFn, RunFnFut, Output, UpdateFn>
where
    RunFn: FnOnce(Arc<Tper>) -> RunFnFut + Send + 'static,
    RunFnFut: Future<Output = Output> + Send,
    Output: Send + 'static,
    UpdateFn: FnOnce(ui::Device, Output) -> ui::Device + 'static,
{
    runtime: Arc<PolyRuntime>,
    device_list: Arc<RwLock<DeviceList>>,
    device_id: PathBuf,
    run_fn: RunFn,
    update_fn: UpdateFn,
}

impl<RunFn, RunFnFut, Output, UpdateFn> UpdateOnTper<RunFn, RunFnFut, Output, UpdateFn>
where
    RunFn: FnOnce(Arc<Tper>) -> RunFnFut + Send + 'static,
    RunFnFut: Future<Output = Output> + Send,
    Output: Send + 'static,
    UpdateFn: FnOnce(ui::Device, Output) -> ui::Device + 'static,
{
    pub fn run(self) {
        let Self { runtime, device_list, device_id, run_fn, update_fn } = self;
        slint::spawn_local(
            async move {
                // Acquire resources.
                let device_list = device_list.read().await;
                let Some(backend) = device_list.backend.get(&device_id) else { return };
                let backend = backend.read().await;
                let Some(tper) = backend.tper.clone() else { return };

                // Indicate to UI that we're busy on the TPer.
                device_list.ui.update(&device_id, |value| {
                    let command_status = value.command_status.clone();
                    value.with_command_status(command_status.with_tper_busy(true))
                });

                // Execute command and update results.
                let output = runtime.spawn(async move { run_fn(tper).await }.in_current_span()).await.unwrap();
                device_list.ui.update(&device_id, move |value| update_fn(value, output));

                // Indicate to UI that we're NO LONGER busy.
                device_list.ui.update(&device_id, |value| {
                    let command_status = value.command_status.clone();
                    value.with_command_status(command_status.with_tper_busy(false))
                });
            }
            .in_current_span(),
        )
        .expect_in_event_loop();
    }
}

//------------------------------------------------------------------------------
// Command on session
//------------------------------------------------------------------------------

pub struct CommandOnSession<RunFn, RunFnFut, Output>
where
    RunFn: FnOnce(Arc<Tper>, Mut<Session>) -> RunFnFut + Send + 'static,
    RunFnFut: Future<Output = Output> + Send,
    Output: Send + 'static,
{
    runtime: Arc<PolyRuntime>,
    device_list: Arc<RwLock<DeviceList>>,
    device_id: PathBuf,
    run_fn: RunFn,
}

impl<RunFn, RunFnFut, Output> CommandOnSession<RunFn, RunFnFut, Output>
where
    RunFn: FnOnce(Arc<Tper>, Mut<Session>) -> RunFnFut + Send + 'static,
    RunFnFut: Future<Output = Output> + Send,
    Output: Send + 'static,
{
    pub fn display<UpdateFn>(self, update_fn: UpdateFn) -> UpdateOnSession<RunFn, RunFnFut, Output, UpdateFn>
    where
        UpdateFn: for<'spec> FnOnce(ui::Device, Option<&'spec Spec>, Output) -> ui::Device + 'static,
    {
        let Self { runtime, device_list, device_id, run_fn } = self;
        UpdateOnSession { runtime, device_list, device_id, run_fn, update_fn }
    }
}

pub struct UpdateOnSession<RunFn, RunFnFut, Output, UpdateFn>
where
    RunFn: FnOnce(Arc<Tper>, Mut<Session>) -> RunFnFut + Send + 'static,
    RunFnFut: Future<Output = Output> + Send,
    Output: Send + 'static,
    UpdateFn: for<'spec> FnOnce(ui::Device, Option<&'spec Spec>, Output) -> ui::Device + 'static,
{
    runtime: Arc<PolyRuntime>,
    device_list: Arc<RwLock<DeviceList>>,
    device_id: PathBuf,
    run_fn: RunFn,
    update_fn: UpdateFn,
}

impl<RunFn, RunFnFut, Output, UpdateFn> UpdateOnSession<RunFn, RunFnFut, Output, UpdateFn>
where
    RunFn: FnOnce(Arc<Tper>, Mut<Session>) -> RunFnFut + Send + 'static,
    RunFnFut: Future<Output = Output> + Send,
    Output: Send + 'static,
    UpdateFn: for<'spec> FnOnce(ui::Device, Option<&'spec Spec>, Output) -> ui::Device + 'static,
{
    pub fn run(self) {
        let Self { runtime, device_list, device_id, run_fn, update_fn } = self;
        slint::spawn_local(
            async move {
                // Acquire resources.
                let device_list = device_list.read().await;
                let Some(backend) = device_list.backend.get(&device_id) else { return };
                let backend = backend.read().await;
                let Some(tper) = backend.tper.clone() else { return };
                let session = backend.session.lock_arc().await;

                // Indicate to UI that we're busy on the session.
                device_list.ui.update(&device_id, |value| {
                    let command_status = value.command_status.clone();
                    value.with_command_status(command_status.with_session_busy(true))
                });

                // Execute the command and display results.
                let output = runtime
                    .spawn(async move { run_fn(tper, Mut::Mutex(session)).await }.in_current_span())
                    .await
                    .unwrap();
                let spec = backend.specification.as_ref();
                device_list.ui.update(&device_id, move |value| update_fn(value, spec, output));

                // Indicate to UI that we're NO LONGER busy.
                device_list.ui.update(&device_id, |value| {
                    let command_status = value.command_status.clone();
                    value.with_command_status(command_status.with_session_busy(false))
                });

                // Update active session.
                let session = backend.session.lock_arc().await;
                device_list.ui.update(&device_id, |mut value| {
                    value.command_status.secondary_session_active = matches!(*session, Session::LockingConfig(_));
                    value
                });
            }
            .in_current_span(),
        )
        .expect_in_event_loop();
    }
}

#[derive(Debug)]
pub enum Mut<T> {
    Mutex(async_lock::MutexGuardArc<T>),
    RwLock(async_lock::RwLockWriteGuardArc<T>),
}

impl<T> Deref for Mut<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Mut::Mutex(inner) => inner.deref(),
            Mut::RwLock(inner) => inner.deref(),
        }
    }
}

impl<T> DerefMut for Mut<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Mut::Mutex(inner) => inner.deref_mut(),
            Mut::RwLock(inner) => inner.deref_mut(),
        }
    }
}

pub(crate) trait ExpectInEventLoop {
    type Output;

    fn expect_in_event_loop(self) -> Self::Output;
}

impl<T> ExpectInEventLoop for Result<T, EventLoopError> {
    type Output = T;
    fn expect_in_event_loop(self) -> Self::Output {
        self.expect("expected to be inside the event loop")
    }
}
