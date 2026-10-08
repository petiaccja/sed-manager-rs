//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::fmt::Display;
use std::ops::DerefMut;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use async_lock::RwLock;
use sed_async::{PolyRuntime, Runtime as _};
use sed_manager::{Device, Error, Spec};
use sed_manager_gui_slint as ui;
use slint::EventLoopError;
use tracing::Instrument;

use crate::device_list::{DeviceEntry, DeviceList};
use crate::session::Session;
use crate::toast::ToastQueue;
use crate::ui_ext::{CommandStatusExt, DeviceExt};

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("The command was skipped.")]
    Skipped,
    #[error(transparent)]
    Failed(#[from] Error),
}

//------------------------------------------------------------------------------
// Command initial
//------------------------------------------------------------------------------

pub struct Command {
    runtime: Arc<PolyRuntime>,
    device_list: Rc<RwLock<DeviceList>>,
    toast_queue: Rc<ToastQueue>,
    success_msg: Option<String>,
    error_msg: Option<String>,
}

impl Command {
    pub fn new(runtime: Arc<PolyRuntime>, device_list: Rc<RwLock<DeviceList>>, toast_queue: Rc<ToastQueue>) -> Self {
        Self { runtime, device_list, toast_queue, success_msg: None, error_msg: None }
    }

    pub fn success_message(self, message: impl Display) -> Self {
        Self { success_msg: Some(message.to_string()), ..self }
    }

    pub fn error_message(self, message: impl Display) -> Self {
        Self { error_msg: Some(message.to_string()), ..self }
    }

    pub fn success_message_if(self, message: Option<impl Display>) -> Self {
        Self { success_msg: message.as_ref().map(ToString::to_string), ..self }
    }

    pub fn error_message_if(self, message: Option<impl Display>) -> Self {
        Self { error_msg: message.as_ref().map(ToString::to_string), ..self }
    }

    pub fn on_device_list<RunFn, Output>(self, run_fn: RunFn) -> CommandOnDeviceList<RunFn, Output>
    where
        RunFn: for<'a> AsyncFnOnce(&'a mut DeviceList) -> Result<Output, CommandError> + 'static,
    {
        let Self { success_msg, error_msg, device_list, toast_queue, .. } = self;
        CommandOnDeviceList {
            device_list,
            toast_queue,
            run_fn,
            update_fn: None,
            error_fn: None,
            finally_fn: None,
            success_msg,
            error_msg,
        }
    }

    pub fn on_device_entry<RunFn, Output>(self, device: PathBuf, run_fn: RunFn) -> CommandOnDeviceEntry<RunFn, Output>
    where
        RunFn: for<'x> OnDeviceEntryRunFn<'x, Output = Output> + Send + 'static,
        for<'x> <RunFn as OnDeviceEntryRunFn<'x>>::Future: Send,
        Output: Send + 'static,
    {
        let Self { runtime, device_list, toast_queue, success_msg, error_msg } = self;
        CommandOnDeviceEntry { success_msg, error_msg, runtime, device_list, device_id: device, run_fn }
    }

    pub fn on_device<RunFn, Output>(self, device_id: PathBuf, run_fn: RunFn) -> CommandOnDevice<RunFn, Output>
    where
        RunFn: for<'x> OnDeviceRunFn<'x, Output = Output> + Send + 'static,
        for<'x> <RunFn as OnDeviceRunFn<'x>>::Future: Send,
        Output: Send + 'static,
    {
        let Self { runtime, device_list, toast_queue, success_msg, error_msg } = self;
        CommandOnDevice { success_msg, error_msg, runtime, device_list, device_id, run_fn }
    }

    pub fn on_session<RunFn, Output>(self, device_id: PathBuf, run_fn: RunFn) -> CommandOnSession<RunFn, Output>
    where
        RunFn: for<'x> OnSessionRunFn<'x, Output = Output> + Send + 'static,
        for<'x> <RunFn as OnSessionRunFn<'x>>::Future: Send,
        Output: Send + 'static,
    {
        let Self { runtime, device_list, toast_queue, success_msg, error_msg } = self;
        CommandOnSession { success_msg, error_msg, runtime, device_list, device_id, run_fn }
    }
}

pub trait OnDeviceEntryRunFn<'x> {
    type Output;
    type Future: Future<Output = Self::Output> + Send;
    fn call_once(self, device_entry: &'x mut DeviceEntry) -> Self::Future;
}

impl<'x, F, Fut, Output> OnDeviceEntryRunFn<'x> for F
where
    F: FnOnce(&'x mut DeviceEntry) -> Fut,
    Fut: Future<Output = Output> + Send + 'x,
{
    type Output = Output;
    type Future = Fut;
    fn call_once(self, device_entry: &'x mut DeviceEntry) -> Fut {
        self(device_entry)
    }
}

pub trait OnDeviceRunFn<'x> {
    type Output;
    type Future: Future<Output = Self::Output> + Send;
    fn call_once(self, device: &'x Device) -> Self::Future;
}

impl<'x, F, Fut, Output> OnDeviceRunFn<'x> for F
where
    F: FnOnce(&'x Device) -> Fut,
    Fut: Future<Output = Output> + Send + 'x,
{
    type Output = Output;
    type Future = Fut;
    fn call_once(self, device: &'x Device) -> Fut {
        self(device)
    }
}

pub trait OnSessionRunFn<'x> {
    type Output;
    type Future: Future<Output = Self::Output> + Send;
    fn call_once(self, device: &'x Device, session: &'x mut Session) -> Self::Future;
}

impl<'x, F, Fut, Output> OnSessionRunFn<'x> for F
where
    F: FnOnce(&'x Device, &'x mut Session) -> Fut,
    Fut: Future<Output = Output> + Send + 'x,
{
    type Output = Output;
    type Future = Fut;
    fn call_once(self, device: &'x Device, session: &'x mut Session) -> Fut {
        self(device, session)
    }
}

//------------------------------------------------------------------------------
// Command on device list
//------------------------------------------------------------------------------

pub struct CommandOnDeviceList<RunFn, Output> {
    device_list: Rc<RwLock<DeviceList>>,
    toast_queue: Rc<ToastQueue>,
    run_fn: RunFn,
    update_fn: Option<Box<dyn FnOnce(&mut DeviceList, Output)>>,
    error_fn: Option<Box<dyn FnOnce(&mut DeviceList, Error)>>,
    finally_fn: Option<Box<dyn FnOnce(&mut DeviceList)>>,
    success_msg: Option<String>,
    error_msg: Option<String>,
}

impl<RunFn, Output> CommandOnDeviceList<RunFn, Output>
where
    RunFn: for<'a> AsyncFnOnce(&'a mut DeviceList) -> Result<Output, CommandError> + 'static,
    Output: 'static,
{
    pub fn on_success<UpdateFn>(self, update_fn: UpdateFn) -> CommandOnDeviceList<RunFn, Output>
    where
        UpdateFn: for<'a> FnOnce(&'a mut DeviceList, Output) + 'static,
    {
        CommandOnDeviceList { update_fn: Some(Box::new(update_fn)), ..self }
    }

    #[allow(unused)]
    pub fn on_error<ErrorFn>(self, error_fn: ErrorFn) -> CommandOnDeviceList<RunFn, Output>
    where
        ErrorFn: for<'a> FnOnce(&'a mut DeviceList, Error) + 'static,
    {
        CommandOnDeviceList { error_fn: Some(Box::new(error_fn)), ..self }
    }

    pub fn finally<FinallyFn>(self, finally_fn: FinallyFn) -> CommandOnDeviceList<RunFn, Output>
    where
        FinallyFn: for<'a> FnOnce(&'a mut DeviceList) + 'static,
    {
        CommandOnDeviceList { finally_fn: Some(Box::new(finally_fn)), ..self }
    }

    #[expect(clippy::option_map_unit_fn)]
    pub fn run(self) {
        let Self { success_msg, error_msg, device_list, toast_queue, run_fn, update_fn, error_fn, finally_fn } = self;
        slint::spawn_local(
            async move {
                let mut device_list = device_list.write().await;
                let result = run_fn(device_list.deref_mut()).await;

                match result {
                    Ok(output) => {
                        success_msg.map(|m| toast_queue.success(m, "".into()));
                        update_fn.map(|f| f(device_list.deref_mut(), output));
                    }
                    Err(CommandError::Failed(err)) => {
                        tracing::error!(error = %err, "command failed");
                        error_msg.map(|m| toast_queue.error(m, err.to_string()));
                        error_fn.map(|f| f(device_list.deref_mut(), err));
                    }
                    Err(CommandError::Skipped) => tracing::debug!("command skipped"),
                };

                finally_fn.map(|f| f(device_list.deref_mut()));
            }
            .in_current_span(),
        )
        .expect_in_event_loop();
    }
}

//------------------------------------------------------------------------------
// Command on device
//------------------------------------------------------------------------------
pub struct CommandOnDeviceEntry<RunFn, Output>
where
    RunFn: for<'x> OnDeviceEntryRunFn<'x, Output = Output> + Send + 'static,
    for<'x> <RunFn as OnDeviceEntryRunFn<'x>>::Future: Send,
    Output: Send + 'static,
{
    success_msg: Option<String>,
    error_msg: Option<String>,
    runtime: Arc<PolyRuntime>,
    device_list: Rc<RwLock<DeviceList>>,
    device_id: PathBuf,
    run_fn: RunFn,
}

impl<RunFn, Output> CommandOnDeviceEntry<RunFn, Output>
where
    RunFn: for<'x> OnDeviceEntryRunFn<'x, Output = Output> + Send + 'static,
    for<'x> <RunFn as OnDeviceEntryRunFn<'x>>::Future: Send,
    Output: Send + 'static,
{
    pub fn display<UpdateFn>(self, update_fn: UpdateFn) -> UpdateOnDeviceEntry<RunFn, Output, UpdateFn>
    where
        UpdateFn: FnOnce(ui::Device, Output) -> ui::Device + 'static,
    {
        let Self { success_msg, error_msg, runtime, device_list, device_id, run_fn } = self;
        UpdateOnDeviceEntry { success_msg, error_msg, runtime, device_list, device_id, run_fn, update_fn }
    }
}

pub struct UpdateOnDeviceEntry<RunFn, Output, UpdateFn>
where
    RunFn: for<'x> OnDeviceEntryRunFn<'x, Output = Output> + Send + 'static,
    for<'x> <RunFn as OnDeviceEntryRunFn<'x>>::Future: Send,
    Output: Send + 'static,
    UpdateFn: FnOnce(ui::Device, Output) -> ui::Device + 'static,
{
    success_msg: Option<String>,
    error_msg: Option<String>,
    runtime: Arc<PolyRuntime>,
    device_list: Rc<RwLock<DeviceList>>,
    device_id: PathBuf,
    run_fn: RunFn,
    update_fn: UpdateFn,
}

impl<RunFn, Output, UpdateFn> UpdateOnDeviceEntry<RunFn, Output, UpdateFn>
where
    RunFn: for<'x> OnDeviceEntryRunFn<'x, Output = Output> + Send + 'static,
    for<'x> <RunFn as OnDeviceEntryRunFn<'x>>::Future: Send,
    Output: Send + 'static,
    UpdateFn: FnOnce(ui::Device, Output) -> ui::Device + 'static,
{
    pub fn run(self) {
        let Self { success_msg, error_msg, runtime, device_list, device_id, run_fn, update_fn } = self;
        slint::spawn_local(
            async move {
                let state = device_list.read().await;
                let Some(device) = state.backend.get(&device_id) else { return };
                let mut device = device.write_arc().await;

                let run_task = runtime
                    .spawn(async move { (run_fn.call_once(device.deref_mut()).await, device) }.in_current_span());

                if let Ok((output, _device)) = run_task.await {
                    state.ui.update(&device_id, move |value| update_fn(value, output));
                }
            }
            .in_current_span(),
        )
        .expect_in_event_loop();
    }
}

//------------------------------------------------------------------------------
// Command on Tper
//------------------------------------------------------------------------------

pub struct CommandOnDevice<RunFn, Output>
where
    RunFn: for<'x> OnDeviceRunFn<'x, Output = Output> + Send + 'static,
    for<'x> <RunFn as OnDeviceRunFn<'x>>::Future: Send,
    Output: Send + 'static,
{
    success_msg: Option<String>,
    error_msg: Option<String>,
    runtime: Arc<PolyRuntime>,
    device_list: Rc<RwLock<DeviceList>>,
    device_id: PathBuf,
    run_fn: RunFn,
}

impl<RunFn, Output> CommandOnDevice<RunFn, Output>
where
    RunFn: for<'x> OnDeviceRunFn<'x, Output = Output> + Send + 'static,
    for<'x> <RunFn as OnDeviceRunFn<'x>>::Future: Send,
    Output: Send + 'static,
{
    pub fn display<UpdateFn>(self, update_fn: UpdateFn) -> UpdateOnDevice<RunFn, Output, UpdateFn>
    where
        UpdateFn: FnOnce(ui::Device, Output) -> ui::Device + 'static,
    {
        let Self { success_msg, error_msg, runtime, device_list, device_id, run_fn } = self;
        UpdateOnDevice { success_msg, error_msg, runtime, device_list, device_id, run_fn, update_fn }
    }
}

pub struct UpdateOnDevice<RunFn, Output, UpdateFn>
where
    RunFn: for<'x> OnDeviceRunFn<'x, Output = Output> + Send + 'static,
    for<'x> <RunFn as OnDeviceRunFn<'x>>::Future: Send,
    Output: Send + 'static,
    UpdateFn: FnOnce(ui::Device, Output) -> ui::Device + 'static,
{
    success_msg: Option<String>,
    error_msg: Option<String>,
    runtime: Arc<PolyRuntime>,
    device_list: Rc<RwLock<DeviceList>>,
    device_id: PathBuf,
    run_fn: RunFn,
    update_fn: UpdateFn,
}

impl<RunFn, Output, UpdateFn> UpdateOnDevice<RunFn, Output, UpdateFn>
where
    RunFn: for<'x> OnDeviceRunFn<'x, Output = Output> + Send + 'static,
    for<'x> <RunFn as OnDeviceRunFn<'x>>::Future: Send,
    Output: Send + 'static,
    UpdateFn: FnOnce(ui::Device, Output) -> ui::Device + 'static,
{
    pub fn run(self) {
        let Self { success_msg, error_msg, runtime, device_list, device_id, run_fn, update_fn } = self;
        slint::spawn_local(
            async move {
                // Acquire resources.
                let device_list = device_list.read().await;
                let Some(device_entry) = device_list.backend.get(&device_id) else { return };
                let device_entry = device_entry.read_arc().await;

                // Indicate to UI that we're busy on the TPer.
                device_list.ui.update(&device_id, |value| {
                    let command_status = value.command_status.clone();
                    value.with_command_status(command_status.with_tper_busy(true))
                });

                // Execute command and update results.
                let run_task = runtime.spawn(
                    async move {
                        let device = device_entry.device.as_ref()?;
                        let output = run_fn.call_once(device).await;
                        Some((output, device_entry))
                    }
                    .in_current_span(),
                );

                if let Some((output, _backend)) = run_task.await.expect("commands should not be cancelled") {
                    // Execute the update fn.
                    device_list.ui.update(&device_id, move |value| update_fn(value, output));

                    // Indicate to UI that we're NO LONGER busy.
                    device_list.ui.update(&device_id, |value| {
                        let command_status = value.command_status.clone();
                        value.with_command_status(command_status.with_tper_busy(false))
                    });
                }
            }
            .in_current_span(),
        )
        .expect_in_event_loop();
    }
}

//------------------------------------------------------------------------------
// Command on session
//------------------------------------------------------------------------------

pub struct CommandOnSession<RunFn, Output>
where
    RunFn: for<'x> OnSessionRunFn<'x, Output = Output> + Send + 'static,
    for<'x> <RunFn as OnSessionRunFn<'x>>::Future: Send,
    Output: Send + 'static,
{
    success_msg: Option<String>,
    error_msg: Option<String>,
    runtime: Arc<PolyRuntime>,
    device_list: Rc<RwLock<DeviceList>>,
    device_id: PathBuf,
    run_fn: RunFn,
}

impl<RunFn, Output> CommandOnSession<RunFn, Output>
where
    RunFn: for<'x> OnSessionRunFn<'x, Output = Output> + Send + 'static,
    for<'x> <RunFn as OnSessionRunFn<'x>>::Future: Send,
    Output: Send + 'static,
{
    pub fn display<UpdateFn>(self, update_fn: UpdateFn) -> UpdateOnSession<RunFn, Output, UpdateFn>
    where
        UpdateFn: for<'spec> FnOnce(ui::Device, Option<&'spec Spec>, Output) -> ui::Device + 'static,
    {
        let Self { success_msg, error_msg, runtime, device_list, device_id, run_fn } = self;
        UpdateOnSession { success_msg, error_msg, runtime, device_list, device_id, run_fn, update_fn }
    }
}

pub struct UpdateOnSession<RunFn, Output, UpdateFn>
where
    RunFn: for<'x> OnSessionRunFn<'x, Output = Output> + Send + 'static,
    for<'x> <RunFn as OnSessionRunFn<'x>>::Future: Send,
    Output: Send + 'static,
    UpdateFn: for<'spec> FnOnce(ui::Device, Option<&'spec Spec>, Output) -> ui::Device + 'static,
{
    success_msg: Option<String>,
    error_msg: Option<String>,
    runtime: Arc<PolyRuntime>,
    device_list: Rc<RwLock<DeviceList>>,
    device_id: PathBuf,
    run_fn: RunFn,
    update_fn: UpdateFn,
}

impl<RunFn, Output, UpdateFn> UpdateOnSession<RunFn, Output, UpdateFn>
where
    RunFn: for<'x> OnSessionRunFn<'x, Output = Output> + Send + 'static,
    for<'x> <RunFn as OnSessionRunFn<'x>>::Future: Send,
    Output: Send + 'static,
    UpdateFn: for<'spec> FnOnce(ui::Device, Option<&'spec Spec>, Output) -> ui::Device + 'static,
{
    pub fn run(self) {
        let Self { success_msg, error_msg, runtime, device_list, device_id, run_fn, update_fn } = self;

        slint::spawn_local(
            async move {
                let device_list = device_list.read().await;
                let Some(device_entry) = device_list.backend.get(&device_id).cloned() else {
                    return;
                };
                let device_entry = device_entry.read_arc().await;

                let (busy_sender, busy_signal) = oneshot::async_channel();
                let run_task = runtime.spawn(
                    async move {
                        // Acquire resources.
                        let device = device_entry.device.as_ref()?;
                        let mut session = device_entry.session.lock_arc().await;

                        // Signal that now we're busy on the session.
                        let _ = busy_sender.send(());

                        // Execute the command and display results.
                        let output = run_fn.call_once(device, session.deref_mut()).await;
                        Some((output, device_entry))
                    }
                    .in_current_span(),
                );

                if busy_signal.await.is_ok() {
                    // Indicate to UI that we're busy on the session.
                    device_list.ui.update(&device_id, |value| {
                        let command_status = value.command_status.clone();
                        value.with_command_status(command_status.with_session_busy(true))
                    });
                }

                if let Some((output, device_entry)) = run_task.await.expect("commands should not be cancelled") {
                    let spec = device_entry.device.as_ref().and_then(|device| device.spec().ok());
                    // Execute the update fn.
                    device_list.ui.update(&device_id, move |value| update_fn(value, spec, output));

                    // Indicate to UI that we're NO LONGER busy.
                    device_list.ui.update(&device_id, |value| {
                        let command_status = value.command_status.clone();
                        value.with_command_status(command_status.with_session_busy(false))
                    });

                    // Update active session.
                    let session = device_entry.session.lock_arc().await;
                    device_list.ui.update(&device_id, |mut value| {
                        value.command_status.secondary_session_active = matches!(*session, Session::LockingConfig(_));
                        value
                    });
                }
            }
            .in_current_span(),
        )
        .expect_in_event_loop();
    }
}

//------------------------------------------------------------------------------
// Helper stuff
//------------------------------------------------------------------------------

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
