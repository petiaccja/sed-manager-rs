use std::rc::Rc;

use slint::{ComponentHandle as _, Model};

use sed_manager::applications::{Error as AppError, MBREditSession};

use crate::backend::{Backend, EditorSession};
use crate::frontend::Frontend;
use crate::ui;
use crate::utility::{into_vec_model, PeekCell};

pub fn init(frontend: &Frontend, num_devices: usize) {
    frontend.with(|window| {
        let mbr_editor_state = window.global::<ui::MBREditorState>();
        let login_status = ui::ExtendedStatus::error("missing callback".into());
        let update_status = ui::ExtendedStatus::success();
        mbr_editor_state.set_login_statuses(into_vec_model(vec![login_status; num_devices]));
        mbr_editor_state.set_update_statuses(into_vec_model(vec![update_status; num_devices]));
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
            set_update_status(&frontend, device_idx, ui::ExtendedStatus::loading());
            let _ = slint::spawn_local(async move {
                let result = set_enabled(backend, device_idx, enabled).await;
                let current = get_value(&frontend, device_idx);
                if let (Ok(_), Some(current)) = (&result, current) {
                    set_value(&frontend, device_idx, ui::MBRControl { enabled, ..current });
                    set_update_status(&frontend, device_idx, ui::ExtendedStatus::success());
                } else {
                    set_update_status(&frontend, device_idx, ui::ExtendedStatus::from_result(result));
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
            set_update_status(&frontend, device_idx, ui::ExtendedStatus::loading());
            let _ = slint::spawn_local(async move {
                let result = set_done(backend, device_idx, done).await;
                let current = get_value(&frontend, device_idx);
                if let (Ok(_), Some(current)) = (&result, current) {
                    set_value(&frontend, device_idx, ui::MBRControl { done, ..current });
                    set_update_status(&frontend, device_idx, ui::ExtendedStatus::success());
                } else {
                    set_update_status(&frontend, device_idx, ui::ExtendedStatus::from_result(result));
                }
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

fn set_login_status(frontend: &Frontend, device_idx: usize, status: ui::ExtendedStatus) {
    frontend.with(|window| {
        let mbr_editor_state = window.global::<ui::MBREditorState>();
        let login_statuses = mbr_editor_state.get_login_statuses();
        if device_idx < login_statuses.row_count() {
            login_statuses.set_row_data(device_idx, status);
        }
    });
}

fn set_update_status(frontend: &Frontend, device_idx: usize, status: ui::ExtendedStatus) {
    frontend.with(|window| {
        let mbr_editor_state = window.global::<ui::MBREditorState>();
        let update_statuses = mbr_editor_state.get_update_statuses();
        if device_idx < update_statuses.row_count() {
            update_statuses.set_row_data(device_idx, status);
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
