use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::sync::Arc;

use async_lock::RwLock;
use sed_async::Runtime;
use sed_manager_gui_slint as ui;
use sed_tper::Tper;
use slint::EventLoopError;
use tracing::Instrument;

use crate::device_list::{Device, DeviceList};
use crate::session::Session;

//------------------------------------------------------------------------------
// Command initial
//------------------------------------------------------------------------------

pub struct Command {
    runtime: Arc<Runtime>,
    state: Arc<RwLock<DeviceList>>,
}

impl Command {
    pub fn new(runtime: Arc<Runtime>, state: Arc<RwLock<DeviceList>>) -> Self {
        Self { runtime, state }
    }

    pub fn on_device_list<RunFn, Output>(self, run_fn: RunFn) -> CommandOnDeviceList<RunFn, Output>
    where
        RunFn: for<'a> AsyncFnOnce(&'a mut DeviceList) -> Output + 'static,
    {
        let Self { state, .. } = self;
        CommandOnDeviceList { state, run_fn }
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
        let Self { runtime, state } = self;
        CommandOnDevice { runtime, state, device_id: device, run_fn }
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
        let Self { runtime, state } = self;
        CommandOnTper { runtime, state, device_id, run_fn }
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
        let Self { runtime, state } = self;
        CommandOnSession { runtime, state, device_id, run_fn }
    }
}

//------------------------------------------------------------------------------
// Command on device list
//------------------------------------------------------------------------------

pub struct CommandOnDeviceList<RunFn, Output>
where
    RunFn: for<'a> AsyncFnOnce(&'a mut DeviceList) -> Output + 'static,
{
    state: Arc<RwLock<DeviceList>>,
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
        let Self { state, run_fn, .. } = self;
        UpdateOnDeviceList { state, run_fn, update_fn }
    }
}

pub struct UpdateOnDeviceList<RunFn, Output, UpdateFn>
where
    RunFn: for<'a> AsyncFnOnce(&'a mut DeviceList) -> Output + 'static,
    UpdateFn: for<'a> FnOnce(&'a mut DeviceList, Output) + 'static,
{
    state: Arc<RwLock<DeviceList>>,
    run_fn: RunFn,
    update_fn: UpdateFn,
}

impl<RunFn, Output, UpdateFn> UpdateOnDeviceList<RunFn, Output, UpdateFn>
where
    RunFn: for<'a> AsyncFnOnce(&'a mut DeviceList) -> Output + 'static,
    UpdateFn: for<'a> FnOnce(&'a mut DeviceList, Output) + 'static,
{
    pub fn run(self) {
        let Self { state, run_fn, update_fn } = self;
        let _ = slint::spawn_local(
            async move {
                let mut state = state.write().await;
                let output = run_fn(state.deref_mut()).await;
                update_fn(state.deref_mut(), output);
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
    runtime: Arc<Runtime>,
    state: Arc<RwLock<DeviceList>>,
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
        let Self { runtime, state, device_id, run_fn } = self;
        UpdateOnDevice { runtime, state, device_id, run_fn, update_fn }
    }
}

pub struct UpdateOnDevice<RunFn, RunFnFut, Output, UpdateFn>
where
    RunFn: FnOnce(Mut<Device>) -> RunFnFut + Send + 'static,
    RunFnFut: Future<Output = Output> + Send,
    Output: Send + 'static,
    UpdateFn: FnOnce(ui::Device, Output) -> ui::Device + 'static,
{
    runtime: Arc<Runtime>,
    state: Arc<RwLock<DeviceList>>,
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
        let Self { runtime, state, device_id, run_fn, update_fn } = self;
        slint::spawn_local(
            async move {
                let state = state.read().await;
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
    runtime: Arc<Runtime>,
    state: Arc<RwLock<DeviceList>>,
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
        let Self { runtime, state, device_id, run_fn } = self;
        UpdateOnTper { runtime, state, device_id, run_fn, update_fn }
    }
}

pub struct UpdateOnTper<RunFn, RunFnFut, Output, UpdateFn>
where
    RunFn: FnOnce(Arc<Tper>) -> RunFnFut + Send + 'static,
    RunFnFut: Future<Output = Output> + Send,
    Output: Send + 'static,
    UpdateFn: FnOnce(ui::Device, Output) -> ui::Device + 'static,
{
    runtime: Arc<Runtime>,
    state: Arc<RwLock<DeviceList>>,
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
        let Self { runtime, state, device_id: device, run_fn, update_fn } = self;
        slint::spawn_local(
            async move {
                let state = state.read().await;
                let Some(backend) = state.backend.get(&device) else { return };
                let backend = backend.read().await;
                let Some(tper) = backend.tper.clone() else { return };
                let output = runtime.spawn(async move { run_fn(tper).await }.in_current_span()).await.unwrap();
                state.ui.update(&device, move |value| update_fn(value, output));
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
    runtime: Arc<Runtime>,
    state: Arc<RwLock<DeviceList>>,
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
        UpdateFn: FnOnce(ui::Device, Output) -> ui::Device + 'static,
    {
        let Self { runtime, state, device_id, run_fn } = self;
        UpdateOnSession { runtime, state, device_id, run_fn, update_fn }
    }
}

pub struct UpdateOnSession<RunFn, RunFnFut, Output, UpdateFn>
where
    RunFn: FnOnce(Arc<Tper>, Mut<Session>) -> RunFnFut + Send + 'static,
    RunFnFut: Future<Output = Output> + Send,
    Output: Send + 'static,
    UpdateFn: FnOnce(ui::Device, Output) -> ui::Device + 'static,
{
    runtime: Arc<Runtime>,
    state: Arc<RwLock<DeviceList>>,
    device_id: PathBuf,
    run_fn: RunFn,
    update_fn: UpdateFn,
}

impl<RunFn, RunFnFut, Output, UpdateFn> UpdateOnSession<RunFn, RunFnFut, Output, UpdateFn>
where
    RunFn: FnOnce(Arc<Tper>, Mut<Session>) -> RunFnFut + Send + 'static,
    RunFnFut: Future<Output = Output> + Send,
    Output: Send + 'static,
    UpdateFn: FnOnce(ui::Device, Output) -> ui::Device + 'static,
{
    pub fn run(self) {
        let Self { runtime, state, device_id: device, run_fn, update_fn } = self;
        slint::spawn_local(
            async move {
                let state = state.read().await;
                let Some(backend) = state.backend.get(&device) else { return };
                let backend = backend.read().await;
                let Some(tper) = backend.tper.clone() else { return };
                let session = backend.session.lock_arc().await;
                let output = runtime
                    .spawn(async move { run_fn(tper, Mut::Mutex(session)).await }.in_current_span())
                    .await
                    .unwrap();
                state.ui.update(&device, move |value| update_fn(value, output));
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
