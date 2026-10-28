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
use sed_manager::{Device, DeviceSpec};
use sed_manager_gui_slint as ui;
use slint::EventLoopError;
use tracing::Instrument;

use crate::Error;
use crate::busy::{BusyGuard, Resource};
use crate::device_list::{DeviceEntry, DeviceList};
use crate::session::Session;
use crate::toast::ToastQueue;

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
        RunFn: for<'a> AsyncFnOnce(&'a mut DeviceList) -> Result<Output, Error> + 'static,
    {
        let Self { success_msg, error_msg, device_list, toast_queue, .. } = self;
        CommandOnDeviceList {
            device_list,
            toast_queue,
            run_fn,
            success_fn: None,
            error_fn: None,
            finally_fn: None,
            success_msg,
            error_msg,
        }
    }

    pub fn on_device_entry<RunFn, Output>(self, device: PathBuf, run_fn: RunFn) -> CommandOnDeviceEntry<RunFn, Output>
    where
        RunFn: for<'x> OnDeviceEntryRunFn<'x, Output = Result<Output, Error>> + Send + 'static,
        for<'x> <RunFn as OnDeviceEntryRunFn<'x>>::Future: Send,
        Output: Send + 'static,
    {
        let Self { runtime, device_list, toast_queue, success_msg, error_msg } = self;
        CommandOnDeviceEntry {
            success_msg,
            error_msg,
            runtime,
            device_list,
            toast_queue,
            device_id: device,
            run_fn,
            success_fn: None,
            error_fn: None,
            finally_fn: None,
        }
    }

    pub fn on_device<RunFn, Output>(self, device_id: PathBuf, run_fn: RunFn) -> CommandOnDevice<RunFn, Output>
    where
        RunFn: for<'x> OnDeviceRunFn<'x, Output = Result<Output, Error>> + Send + 'static,
        for<'x> <RunFn as OnDeviceRunFn<'x>>::Future: Send,
        Output: Send + 'static,
    {
        let Self { runtime, device_list, toast_queue, success_msg, error_msg } = self;
        CommandOnDevice {
            success_msg,
            error_msg,
            runtime,
            device_list,
            toast_queue,
            device_id,
            run_fn,
            success_fn: None,
            error_fn: None,
            finally_fn: None,
        }
    }

    pub fn on_session<RunFn, Output>(self, device_id: PathBuf, run_fn: RunFn) -> CommandOnSession<RunFn, Output>
    where
        RunFn: for<'x> OnSessionRunFn<'x, Output = Result<Output, Error>> + Send + 'static,
        for<'x> <RunFn as OnSessionRunFn<'x>>::Future: Send,
        Output: Send + 'static,
    {
        let Self { runtime, device_list, toast_queue, success_msg, error_msg } = self;
        CommandOnSession {
            success_msg,
            error_msg,
            runtime,
            device_list,
            toast_queue,
            device_id,
            run_fn,
            success_fn: None,
            error_fn: None,
            finally_fn: None,
        }
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
    success_fn: Option<Box<dyn FnOnce(&mut DeviceList, Output)>>,
    error_fn: Option<Box<dyn FnOnce(&mut DeviceList, Error)>>,
    finally_fn: Option<Box<dyn FnOnce(&mut DeviceList)>>,
    success_msg: Option<String>,
    error_msg: Option<String>,
}

impl<RunFn, Output> CommandOnDeviceList<RunFn, Output>
where
    RunFn: for<'a> AsyncFnOnce(&'a mut DeviceList) -> Result<Output, Error> + 'static,
    Output: 'static,
{
    pub fn on_success<SuccessFn>(self, success_fn: SuccessFn) -> Self
    where
        SuccessFn: for<'a> FnOnce(&'a mut DeviceList, Output) + 'static,
    {
        Self { success_fn: Some(Box::new(success_fn)), ..self }
    }

    #[expect(unused)]
    pub fn on_error<ErrorFn>(self, error_fn: ErrorFn) -> Self
    where
        ErrorFn: for<'a> FnOnce(&'a mut DeviceList, Error) + 'static,
    {
        Self { error_fn: Some(Box::new(error_fn)), ..self }
    }

    pub fn finally<FinallyFn>(self, finally_fn: FinallyFn) -> Self
    where
        FinallyFn: for<'a> FnOnce(&'a mut DeviceList) + 'static,
    {
        Self { finally_fn: Some(Box::new(finally_fn)), ..self }
    }

    #[expect(clippy::option_map_unit_fn)]
    pub fn run(self) {
        let Self { success_msg, error_msg, device_list, toast_queue, run_fn, success_fn, error_fn, finally_fn } = self;
        slint::spawn_local(
            async move {
                let mut device_list = device_list.write().await;
                let result = run_fn(device_list.deref_mut()).await;

                match result {
                    Ok(output) => {
                        success_msg.map(|m| toast_queue.success(m, ""));
                        success_fn.map(|f| f(device_list.deref_mut(), output));
                    }
                    Err(Error::CommandSkipped) => tracing::debug!("command skipped"),
                    Err(err) => {
                        tracing::error!(error = %err);
                        error_msg.map(|m| toast_queue.error(m, err.to_string()));
                        error_fn.map(|f| f(device_list.deref_mut(), err));
                    }
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
pub struct CommandOnDeviceEntry<RunFn, Output> {
    runtime: Arc<PolyRuntime>,
    device_list: Rc<RwLock<DeviceList>>,
    toast_queue: Rc<ToastQueue>,
    device_id: PathBuf,
    run_fn: RunFn,
    success_fn: Option<Box<dyn FnOnce(ui::Device, Output) -> ui::Device>>,
    error_fn: Option<Box<dyn FnOnce(ui::Device, Error) -> ui::Device>>,
    finally_fn: Option<Box<dyn FnOnce(ui::Device) -> ui::Device>>,
    success_msg: Option<String>,
    error_msg: Option<String>,
}

impl<RunFn, Output> CommandOnDeviceEntry<RunFn, Output>
where
    RunFn: for<'x> OnDeviceEntryRunFn<'x, Output = Result<Output, Error>> + Send + 'static,
    for<'x> <RunFn as OnDeviceEntryRunFn<'x>>::Future: Send,
    Output: Send + 'static,
{
    pub fn on_success<SuccessFn>(self, success_fn: SuccessFn) -> Self
    where
        SuccessFn: FnOnce(ui::Device, Output) -> ui::Device + 'static,
    {
        Self { success_fn: Some(Box::new(success_fn)), ..self }
    }

    pub fn on_error<ErrorFn>(self, error_fn: ErrorFn) -> Self
    where
        ErrorFn: FnOnce(ui::Device, Error) -> ui::Device + 'static,
    {
        Self { error_fn: Some(Box::new(error_fn)), ..self }
    }

    #[expect(unused)]
    pub fn finally<FinallyFn>(self, finally_fn: FinallyFn) -> Self
    where
        FinallyFn: FnOnce(ui::Device) -> ui::Device + 'static,
    {
        Self { finally_fn: Some(Box::new(finally_fn)), ..self }
    }

    pub fn run(self) {
        let Self {
            success_msg,
            error_msg,
            runtime,
            device_list,
            toast_queue,
            device_id,
            run_fn,
            success_fn,
            error_fn,
            finally_fn,
        } = self;

        slint::spawn_local(
            async move {
                let state = device_list.read().await;
                let Some(device) = state.backend.get(&device_id) else { return };
                let mut device = device.write_arc().await;

                let run_task = runtime
                    .spawn(async move { (run_fn.call_once(device.deref_mut()).await, device) }.in_current_span());

                if let Ok((result, _device)) = run_task.await {
                    state.ui.update(&device_id, move |value| {
                        process_result(
                            result,
                            toast_queue,
                            success_fn,
                            error_fn,
                            finally_fn,
                            value,
                            success_msg,
                            error_msg,
                        )
                    });
                }
            }
            .in_current_span(),
        )
        .expect_in_event_loop();
    }
}

#[expect(clippy::too_many_arguments, reason = "fix it if possible")]
#[expect(clippy::option_map_unit_fn)]
fn process_result<Output>(
    result: Result<Output, Error>,
    toast_queue: Rc<ToastQueue>,
    success_fn: Option<Box<dyn FnOnce(ui::Device, Output) -> ui::Device + 'static>>,
    error_fn: Option<Box<dyn FnOnce(ui::Device, Error) -> ui::Device + 'static>>,
    finally_fn: Option<Box<dyn FnOnce(ui::Device) -> ui::Device + 'static>>,
    value: ui::Device,
    success_msg: Option<String>,
    error_msg: Option<String>,
) -> ui::Device {
    let value = match result {
        Ok(output) => {
            success_msg.map(|m| toast_queue.success(m, ""));
            if let Some(f) = success_fn { f(value, output) } else { value }
        }
        Err(Error::CommandSkipped) => {
            tracing::debug!("command skipped");
            value
        }
        Err(err) => {
            tracing::error!(error = %err);
            error_msg.map(|m| toast_queue.error(m, err.to_string()));
            if let Some(f) = error_fn { f(value, err) } else { value }
        }
    };
    if let Some(f) = finally_fn { f(value) } else { value }
}

//------------------------------------------------------------------------------
// Command on Tper
//------------------------------------------------------------------------------

pub struct CommandOnDevice<RunFn, Output> {
    runtime: Arc<PolyRuntime>,
    device_list: Rc<RwLock<DeviceList>>,
    toast_queue: Rc<ToastQueue>,
    device_id: PathBuf,
    run_fn: RunFn,
    success_fn: Option<Box<dyn FnOnce(ui::Device, Output) -> ui::Device>>,
    error_fn: Option<Box<dyn FnOnce(ui::Device, Error) -> ui::Device>>,
    finally_fn: Option<Box<dyn FnOnce(ui::Device) -> ui::Device>>,
    success_msg: Option<String>,
    error_msg: Option<String>,
}

impl<RunFn, Output> CommandOnDevice<RunFn, Output>
where
    RunFn: for<'x> OnDeviceRunFn<'x, Output = Result<Output, Error>> + Send + 'static,
    for<'x> <RunFn as OnDeviceRunFn<'x>>::Future: Send,
    Output: Send + 'static,
{
    pub fn on_success<SuccessFn>(self, success_fn: SuccessFn) -> Self
    where
        SuccessFn: FnOnce(ui::Device, Output) -> ui::Device + 'static,
    {
        Self { success_fn: Some(Box::new(success_fn)), ..self }
    }

    pub fn on_error<ErrorFn>(self, error_fn: ErrorFn) -> Self
    where
        ErrorFn: FnOnce(ui::Device, Error) -> ui::Device + 'static,
    {
        Self { error_fn: Some(Box::new(error_fn)), ..self }
    }

    #[expect(unused)]
    pub fn finally<FinallyFn>(self, finally_fn: FinallyFn) -> Self
    where
        FinallyFn: FnOnce(ui::Device) -> ui::Device + 'static,
    {
        Self { finally_fn: Some(Box::new(finally_fn)), ..self }
    }

    pub fn run(self) {
        let Self {
            success_msg,
            error_msg,
            runtime,
            device_list,
            toast_queue,
            device_id,
            run_fn,
            success_fn,
            error_fn,
            finally_fn,
        } = self;

        slint::spawn_local(
            async move {
                // Acquire resources.
                let device_list = device_list.read().await;
                let Some(device_entry) = device_list.backend.get(&device_id) else { return };

                // Acquire the lock to the device entry and immediately after
                // indicate to the UI that we're busy on the TPer.
                let (device_entry, busy) =
                    (device_entry.read_arc().await, BusyGuard::new(&device_list, &device_id, Resource::Tper));

                // Apply changes to the device.
                let run_task = runtime.spawn(
                    async move {
                        // The device entry guard is always returned so that the UI thread
                        // releases it, after clearing the busy flag.
                        let output = match device_entry.device.as_ref() {
                            Some(device) => Some(run_fn.call_once(device).await),
                            None => None,
                        };
                        (output, device_entry)
                    }
                    .in_current_span(),
                );

                let (result, device_entry) = run_task.await.expect("commands should not be cancelled");

                // Display results on the UI.
                if let Some(result) = result {
                    device_list.ui.update(&device_id, move |value| {
                        process_result(
                            result,
                            toast_queue,
                            success_fn,
                            error_fn,
                            finally_fn,
                            value,
                            success_msg,
                            error_msg,
                        )
                    });
                }

                // Indicate to UI that we're NO LONGER busy, then immediately
                // after release the device entry lock. The explicit drop is to
                // ensure drop order.
                drop((busy, device_entry));
            }
            .in_current_span(),
        )
        .expect_in_event_loop();
    }
}

//------------------------------------------------------------------------------
// Command on session
//------------------------------------------------------------------------------

pub struct CommandOnSession<RunFn, Output> {
    runtime: Arc<PolyRuntime>,
    device_list: Rc<RwLock<DeviceList>>,
    toast_queue: Rc<ToastQueue>,
    device_id: PathBuf,
    run_fn: RunFn,
    success_fn: Option<Box<dyn FnOnce(ui::Device, Option<&DeviceSpec>, Output) -> ui::Device>>,
    error_fn: Option<Box<dyn FnOnce(ui::Device, Option<&DeviceSpec>, Error) -> ui::Device>>,
    finally_fn: Option<Box<dyn FnOnce(ui::Device) -> ui::Device>>,
    success_msg: Option<String>,
    error_msg: Option<String>,
}

impl<RunFn, Output> CommandOnSession<RunFn, Output>
where
    RunFn: for<'x> OnSessionRunFn<'x, Output = Result<Output, Error>> + Send + 'static,
    for<'x> <RunFn as OnSessionRunFn<'x>>::Future: Send,
    Output: Send + 'static,
{
    pub fn on_success<SuccessFn>(self, success_fn: SuccessFn) -> Self
    where
        SuccessFn: FnOnce(ui::Device, Option<&DeviceSpec>, Output) -> ui::Device + 'static,
    {
        Self { success_fn: Some(Box::new(success_fn)), ..self }
    }

    pub fn on_error<ErrorFn>(self, error_fn: ErrorFn) -> Self
    where
        ErrorFn: FnOnce(ui::Device, Option<&DeviceSpec>, Error) -> ui::Device + 'static,
    {
        Self { error_fn: Some(Box::new(error_fn)), ..self }
    }

    pub fn finally<FinallyFn>(self, finally_fn: FinallyFn) -> Self
    where
        FinallyFn: FnOnce(ui::Device) -> ui::Device + 'static,
    {
        Self { finally_fn: Some(Box::new(finally_fn)), ..self }
    }

    pub fn run(self) {
        let Self {
            runtime,
            device_list,
            toast_queue,
            device_id,
            run_fn,
            success_fn,
            error_fn,
            finally_fn,
            success_msg,
            error_msg,
        } = self;

        slint::spawn_local(
            async move {
                // Acquire resources.
                let device_list = device_list.read().await;
                let Some(device_entry) = device_list.backend.get(&device_id).cloned() else {
                    return;
                };
                let device_entry = device_entry.read_arc().await;

                // Acquire the lock to the session and immediately after
                // indicate to the UI that we're busy on the session.
                let (session, busy) = (
                    device_entry.session.lock_arc().await,
                    BusyGuard::new(&device_list, &device_id, Resource::Session),
                );

                // Apply changes to the device.
                let run_task = runtime.spawn(
                    async move {
                        // The session guard is always returned so that the UI thread
                        // releases it, after clearing the busy flag.
                        let mut session = session;
                        let output = match device_entry.device.as_ref() {
                            Some(device) => Some(run_fn.call_once(device, session.deref_mut()).await),
                            None => None,
                        };
                        (output, device_entry, session)
                    }
                    .in_current_span(),
                );

                let (result, device_entry, session) = run_task.await.expect("commands should not be cancelled");

                // Display results on the UI.
                if let Some(result) = result {
                    let spec = device_entry.device.as_ref().and_then(|device| device.spec().ok());
                    device_list.ui.update(&device_id, move |value| {
                        process_result_with_spec(
                            result,
                            spec,
                            toast_queue,
                            success_fn,
                            error_fn,
                            finally_fn,
                            value,
                            success_msg,
                            error_msg,
                        )
                    });
                }

                // Update active session.
                let secondary_session_active = matches!(*session, Session::LockingConfig(_));
                device_list.ui.update(&device_id, |mut value| {
                    value.command_status.secondary_session_active = secondary_session_active;
                    value
                });

                // Indicate to UI that we're NO LONGER busy, then immediately
                // after release the session lock. The explicit drop is to
                // ensure drop order.
                drop((busy, session));
            }
            .in_current_span(),
        )
        .expect_in_event_loop();
    }
}

#[expect(clippy::option_map_unit_fn)]
#[expect(clippy::too_many_arguments, reason = "not critical, but fix it possible")]
fn process_result_with_spec<Output>(
    result: Result<Output, Error>,
    spec: Option<&DeviceSpec>,
    toast_queue: Rc<ToastQueue>,
    success_fn: Option<Box<dyn FnOnce(ui::Device, Option<&DeviceSpec>, Output) -> ui::Device + 'static>>,
    error_fn: Option<Box<dyn FnOnce(ui::Device, Option<&DeviceSpec>, Error) -> ui::Device + 'static>>,
    finally_fn: Option<Box<dyn FnOnce(ui::Device) -> ui::Device + 'static>>,
    value: ui::Device,
    success_msg: Option<String>,
    error_msg: Option<String>,
) -> ui::Device {
    let value = match result {
        Ok(output) => {
            success_msg.map(|m| toast_queue.success(m, ""));
            if let Some(f) = success_fn { f(value, spec, output) } else { value }
        }
        Err(Error::CommandSkipped) => {
            tracing::debug!("command skipped");
            value
        }
        Err(err) => {
            tracing::error!(error = %err);
            error_msg.map(|m| toast_queue.error(m, err.to_string()));
            if let Some(f) = error_fn { f(value, spec, err) } else { value }
        }
    };
    if let Some(f) = finally_fn { f(value) } else { value }
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
