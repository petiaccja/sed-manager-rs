use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use slint::{ComponentHandle as _, Model};

use sed_manager::applications::{Error as AppError, MBREditSession};
use tokio::io::AsyncReadExt;

use crate::backend::{Backend, EditorSession};
use crate::frontend::Frontend;
use crate::ui;
use crate::utility::{into_vec_model, PeekCell};

pub fn init(frontend: &Frontend, num_devices: usize) {
    frontend.with(|window| {
        let mbr_editor_state = window.global::<ui::MBREditorState>();
        let login_status = ui::ExtendedStatus::error("missing callback".into());
        let control_status = ui::ExtendedStatus::success();
        let upload_status = ui::ExtendedStatus::success();
        mbr_editor_state.set_login_statuses(into_vec_model(vec![login_status; num_devices]));
        mbr_editor_state.set_control_statuses(into_vec_model(vec![control_status; num_devices]));
        mbr_editor_state.set_upload_statuses(into_vec_model(vec![upload_status; num_devices]));
        mbr_editor_state.set_upload_progresses(into_vec_model(vec![0.0; num_devices]));
        mbr_editor_state.set_upload_cancel_reqs(into_vec_model(vec![false; num_devices]));
        mbr_editor_state.set_mbr_control(into_vec_model(vec![ui::MBRControl::default(); num_devices]));
    });
}

pub fn clear(frontend: &Frontend) {
    init(frontend, 0);
}

pub fn set_callbacks(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    set_callback_login(backend.clone(), frontend.clone());
    set_callback_query(backend.clone(), frontend.clone());
    set_callback_set_enabled(backend.clone(), frontend.clone());
    set_callback_set_done(backend.clone(), frontend.clone());
    set_callback_upload(backend.clone(), frontend.clone());
}

fn set_callback_login(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    frontend.clone().with(|window| {
        let mbr_editor_state = window.global::<ui::MBREditorState>();

        mbr_editor_state.on_login(move |device_idx, password| {
            let frontend = frontend.clone();
            let backend = backend.clone();
            let device_idx = device_idx as usize;
            let password = String::from(password);
            set_login_status(&frontend, device_idx, ui::ExtendedStatus::loading());
            let _ = slint::spawn_local(async move {
                let result = login(backend, device_idx, password).await;
                set_login_status(&frontend, device_idx, ui::ExtendedStatus::from_result(result));
            });
        });
    });
}

fn set_callback_query(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    frontend.clone().with(|window| {
        let mbr_editor_state = window.global::<ui::MBREditorState>();

        mbr_editor_state.on_query(move |device_idx| {
            let frontend = frontend.clone();
            let backend = backend.clone();
            let device_idx = device_idx as usize;
            set_login_status(&frontend, device_idx, ui::ExtendedStatus::loading());
            let _ = slint::spawn_local(async move {
                let result = query(backend, device_idx).await;
                set_query(&frontend, device_idx, result);
            });
        });
    });
}

fn set_callback_set_enabled(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    frontend.clone().with(|window| {
        let mbr_editor_state = window.global::<ui::MBREditorState>();

        mbr_editor_state.on_set_enabled(move |device_idx, enabled| {
            let frontend = frontend.clone();
            let backend = backend.clone();
            let device_idx = device_idx as usize;
            set_control_status(&frontend, device_idx, ui::ExtendedStatus::loading());
            let _ = slint::spawn_local(async move {
                let result = set_enabled(backend, device_idx, enabled).await;
                let current = get_value(&frontend, device_idx);
                if let (Ok(_), Some(current)) = (&result, current) {
                    set_value(&frontend, device_idx, ui::MBRControl { enabled, ..current });
                    set_control_status(&frontend, device_idx, ui::ExtendedStatus::success());
                } else {
                    set_control_status(&frontend, device_idx, ui::ExtendedStatus::from_result(result));
                }
            });
        });
    });
}

fn set_callback_set_done(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    frontend.clone().with(|window| {
        let mbr_editor_state = window.global::<ui::MBREditorState>();

        mbr_editor_state.on_set_done(move |device_idx, done| {
            let frontend = frontend.clone();
            let backend = backend.clone();
            let device_idx = device_idx as usize;
            set_control_status(&frontend, device_idx, ui::ExtendedStatus::loading());
            let _ = slint::spawn_local(async move {
                let result = set_done(backend, device_idx, done).await;
                let current = get_value(&frontend, device_idx);
                if let (Ok(_), Some(current)) = (&result, current) {
                    set_value(&frontend, device_idx, ui::MBRControl { done, ..current });
                    set_control_status(&frontend, device_idx, ui::ExtendedStatus::success());
                } else {
                    set_control_status(&frontend, device_idx, ui::ExtendedStatus::from_result(result));
                }
            });
        });
    });
}

fn set_callback_upload(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    frontend.clone().with(|window| {
        let mbr_editor_state = window.global::<ui::MBREditorState>();

        mbr_editor_state.on_upload(move |device_idx| {
            let frontend = frontend.clone();
            let backend = backend.clone();
            let device_idx = device_idx as usize;
            set_upload_status(&frontend, device_idx, ui::ExtendedStatus::loading());
            let _ = slint::spawn_local(async move {
                let result = upload(backend, frontend.clone(), device_idx).await;
                set_upload_progress(&frontend, device_idx, 0.0);
                set_upload_status(&frontend, device_idx, ui::ExtendedStatus::from_result(result));
            });
        });
    });
}

async fn login(backend: Rc<PeekCell<Backend>>, device_idx: usize, password: String) -> Result<(), AppError> {
    let tper = backend.peek_mut(|backend| backend.get_tper(device_idx))?;
    let session = MBREditSession::start(&tper, password.as_bytes()).await?;
    let editor_session = EditorSession::from(session);
    backend.peek_mut(|backend| backend.replace_session(device_idx, editor_session));
    Ok(())
}

async fn query(backend: Rc<PeekCell<Backend>>, device_idx: usize) -> Result<ui::MBRControl, AppError> {
    let session = backend.peek(|backend| backend.get_mbr_session(device_idx))?;
    let size = session.get_size().await?;
    let enabled = session.get_enabled().await?;
    let done = session.get_done().await?;
    Ok(ui::MBRControl { done, enabled, size: size as i32 })
}

async fn set_enabled(backend: Rc<PeekCell<Backend>>, device_idx: usize, enabled: bool) -> Result<(), AppError> {
    let session = backend.peek(|backend| backend.get_mbr_session(device_idx))?;
    Ok(session.set_enabled(enabled).await?)
}

async fn set_done(backend: Rc<PeekCell<Backend>>, device_idx: usize, done: bool) -> Result<(), AppError> {
    let session = backend.peek(|backend| backend.get_mbr_session(device_idx))?;
    Ok(session.set_done(done).await?)
}

async fn upload(backend: Rc<PeekCell<Backend>>, frontend: Frontend, device_idx: usize) -> Result<(), AppError> {
    let Some(file_handle) = rfd::AsyncFileDialog::new().pick_file().await else {
        return Ok(());
    };
    let Ok(runtime) = tokio::runtime::Builder::new_multi_thread().enable_all().build() else {
        return Err(AppError::InternalError);
    };
    let session = backend.peek(|backend| backend.get_mbr_session(device_idx))?;
    let progress_per_mil = Arc::new(AtomicU32::new(0));
    let cancel_req = Arc::new(AtomicBool::new(false));
    let worker_task = runtime.spawn(upload_worker(session, file_handle, progress_per_mil.clone(), cancel_req.clone()));
    let display_callback =
        move || upload_display(frontend.clone(), device_idx, progress_per_mil.clone(), cancel_req.clone());

    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::Repeated, Duration::from_millis(16), display_callback);
    let Ok(result) = worker_task.await else {
        return Err(AppError::InternalError);
    };
    timer.stop();
    result
}

async fn upload_worker(
    session: Arc<MBREditSession>,
    file_handle: rfd::FileHandle,
    progress_per_mil: Arc<AtomicU32>,
    cancel_req: Arc<AtomicBool>,
) -> Result<(), AppError> {
    let Ok(mut file) = tokio::fs::OpenOptions::new().read(true).open(file_handle.path()).await else {
        return Err(AppError::FileNotOpen);
    };
    let len = file.metadata().await.map(|metadata| metadata.len()).unwrap_or(u64::MAX);
    let read = async move |chunk: &mut [u8]| file.read(chunk).await.map_err(|_| AppError::FileReadError);
    let progress = |written| progress_per_mil.store((written * 1000 / len) as u32, Ordering::Relaxed);
    let cancelled = || cancel_req.load(Ordering::Relaxed);
    session.upload(read, progress, cancelled).await
}

fn upload_display(
    frontend: Frontend,
    device_idx: usize,
    progress_per_mil: Arc<AtomicU32>,
    cancel_req: Arc<AtomicBool>,
) {
    let progress = progress_per_mil.load(Ordering::Relaxed) as f32 / 1000.0;
    set_upload_progress(&frontend, device_idx, progress);
    let cancel = get_cancel_req(&frontend, device_idx);
    cancel_req.store(cancel, Ordering::Relaxed);
}

fn set_login_status(frontend: &Frontend, device_idx: usize, status: ui::ExtendedStatus) {
    frontend.with(|window| {
        let mbr_editor_state = window.global::<ui::MBREditorState>();
        let login_statuses = mbr_editor_state.get_login_statuses();
        if device_idx < login_statuses.row_count() {
            login_statuses.set_row_data(device_idx, status);
        }
    });
}

fn set_control_status(frontend: &Frontend, device_idx: usize, status: ui::ExtendedStatus) {
    frontend.with(|window| {
        let mbr_editor_state = window.global::<ui::MBREditorState>();
        let control_statuses = mbr_editor_state.get_control_statuses();
        if device_idx < control_statuses.row_count() {
            control_statuses.set_row_data(device_idx, status);
        }
    });
}

fn set_upload_status(frontend: &Frontend, device_idx: usize, status: ui::ExtendedStatus) {
    frontend.with(|window| {
        let mbr_editor_state = window.global::<ui::MBREditorState>();
        let upload_statuses = mbr_editor_state.get_upload_statuses();
        if device_idx < upload_statuses.row_count() {
            upload_statuses.set_row_data(device_idx, status);
        }
    });
}

fn set_upload_progress(frontend: &Frontend, device_idx: usize, progress: f32) {
    frontend.with(|window| {
        let mbr_editor_state = window.global::<ui::MBREditorState>();
        let upload_progress = mbr_editor_state.get_upload_progresses();
        if device_idx < upload_progress.row_count() {
            upload_progress.set_row_data(device_idx, progress);
        }
    });
}

fn set_query(frontend: &Frontend, device_idx: usize, result: Result<ui::MBRControl, AppError>) {
    if let Ok(value) = result {
        set_value(frontend, device_idx, value);
        set_login_status(frontend, device_idx, ui::ExtendedStatus::success());
    } else {
        set_login_status(frontend, device_idx, ui::ExtendedStatus::from_result(result));
    }
}

fn set_value(frontend: &Frontend, device_idx: usize, value: ui::MBRControl) {
    frontend.with(|window| {
        let mbr_editor_state = window.global::<ui::MBREditorState>();
        let mbr_control = mbr_editor_state.get_mbr_control();
        if device_idx < mbr_control.row_count() {
            mbr_control.set_row_data(device_idx, value);
        }
    });
}

fn get_value(frontend: &Frontend, device_idx: usize) -> Option<ui::MBRControl> {
    frontend
        .with(|window| {
            let mbr_editor_state = window.global::<ui::MBREditorState>();
            let mbr_control = mbr_editor_state.get_mbr_control();
            mbr_control.row_data(device_idx)
        })
        .flatten()
}

fn get_cancel_req(frontend: &Frontend, device_idx: usize) -> bool {
    frontend
        .with(|window| {
            let mbr_editor_state = window.global::<ui::MBREditorState>();
            let cancel_reqs = mbr_editor_state.get_upload_cancel_reqs();
            cancel_reqs.row_data(device_idx)
        })
        .flatten()
        .unwrap_or(false)
}
