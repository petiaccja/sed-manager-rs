//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::rc::Rc;

use slint::{ComponentHandle as _, Model};

use sed_manager::applications::{get_locking_sp, Error as AppError, UserEditSession};

use crate::backend::{get_object_name, Backend, EditorSession};
use crate::frontend::Frontend;
use crate::ui;
use crate::utility::{as_vec_model, into_vec_model, PeekCell};

pub fn init(frontend: &Frontend, num_devices: usize) {
    frontend.with(|window| {
        let user_editor_state = window.global::<ui::UserEditorState>();
        let initial_status = ui::ExtendedStatus::error("missing callback".into());
        user_editor_state.set_login_statuses(into_vec_model(vec![initial_status; num_devices]));
        user_editor_state.set_user_lists(into_vec_model(vec![ui::UserList::empty(); num_devices]));
    });
}

pub fn clear(frontend: &Frontend) {
    init(frontend, 0);
}

pub fn set_callbacks(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    set_callback_login(backend.clone(), frontend.clone());
    set_callback_list(backend.clone(), frontend.clone());
    set_callback_set_enabled(backend.clone(), frontend.clone());
    set_callback_set_name(backend.clone(), frontend.clone());
    set_callback_set_password(backend.clone(), frontend.clone());
}

fn set_callback_login(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    frontend.clone().with(|window| {
        let user_editor_state = window.global::<ui::UserEditorState>();

        user_editor_state.on_login(move |device_idx, password| {
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

fn set_callback_list(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    frontend.clone().with(|window| {
        let user_editor_state = window.global::<ui::UserEditorState>();

        user_editor_state.on_list(move |device_idx| {
            let frontend = frontend.clone();
            let backend = backend.clone();
            let device_idx = device_idx as usize;
            clear_users(&frontend, device_idx);
            set_login_status(&frontend, device_idx, ui::ExtendedStatus::loading());
            let _ = slint::spawn_local(async move {
                let frontend_ = frontend.clone();
                let on_found = move |name, value| {
                    push_user(&frontend, device_idx, name, value);
                };
                let result = list(backend, device_idx, on_found).await;
                set_login_status(&frontend_, device_idx, ui::ExtendedStatus::from_result(result));
            });
        });
    });
}

fn set_callback_set_enabled(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    frontend.clone().with(|window| {
        let user_editor_state = window.global::<ui::UserEditorState>();

        user_editor_state.on_set_enabled(move |device_idx, user_idx, enabled| {
            let frontend = frontend.clone();
            let backend = backend.clone();
            let device_idx = device_idx as usize;
            let user_idx = user_idx as usize;
            set_user_status(&frontend, device_idx, user_idx, ui::ExtendedStatus::loading());
            let _ = slint::spawn_local(async move {
                let Some(mut value) = get_user_value(&frontend, device_idx, user_idx) else {
                    return;
                };
                value.enabled = enabled;
                let result = set_enabled(backend, device_idx, user_idx, enabled).await.map(|_| value);
                set_user(&frontend, device_idx, user_idx, result);
            });
        });
    });
}

fn set_callback_set_name(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    frontend.clone().with(|window| {
        let user_editor_state = window.global::<ui::UserEditorState>();

        user_editor_state.on_set_name(move |device_idx, user_idx, name| {
            let frontend = frontend.clone();
            let backend = backend.clone();
            let device_idx = device_idx as usize;
            let user_idx = user_idx as usize;
            set_user_status(&frontend, device_idx, user_idx, ui::ExtendedStatus::loading());
            let _ = slint::spawn_local(async move {
                let Some(mut value) = get_user_value(&frontend, device_idx, user_idx) else {
                    return;
                };
                value.name = name.clone();
                let result = set_name(backend, device_idx, user_idx, name.into()).await.map(|_| value);
                set_user(&frontend, device_idx, user_idx, result);
            });
        });
    });
}

fn set_callback_set_password(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    frontend.clone().with(|window| {
        let user_editor_state = window.global::<ui::UserEditorState>();

        user_editor_state.on_set_password(move |device_idx, user_idx, password| {
            let frontend = frontend.clone();
            let backend = backend.clone();
            let device_idx = device_idx as usize;
            let user_idx = user_idx as usize;
            set_user_status(&frontend, device_idx, user_idx, ui::ExtendedStatus::loading());
            let _ = slint::spawn_local(async move {
                let result = set_password(backend, device_idx, user_idx, password.into()).await;
                set_user_status(&frontend, device_idx, user_idx, ui::ExtendedStatus::from_result(result));
            });
        });
    });
}

async fn login(backend: Rc<PeekCell<Backend>>, device_idx: usize, password: String) -> Result<(), AppError> {
    let tper = backend.peek_mut(|backend| backend.get_tper(device_idx))?;
    let session = UserEditSession::start(&tper, password.as_bytes()).await?;
    let editor_session = EditorSession::from(session);
    backend.peek_mut(|backend| backend.replace_session(device_idx, editor_session));
    Ok(())
}

async fn list(
    backend: Rc<PeekCell<Backend>>,
    device_idx: usize,
    on_found: impl Fn(String, ui::User),
) -> Result<(), AppError> {
    let session = backend.peek(|backend| backend.get_user_session(device_idx))?;
    let discovery = backend.peek(|backend| backend.get_discovery(device_idx).cloned())?;
    let ssc = discovery.get_primary_ssc().ok_or(AppError::NoAvailableSSC)?;
    let locking_sp = get_locking_sp(ssc.feature_code());
    let users: Vec<_> = session.list_users().await?;
    backend.peek_mut(|backend| backend.set_user_list(device_idx, users.clone()))?;
    for user in users.iter() {
        let name = get_object_name(Some(&discovery), user.as_uid(), locking_sp.clone().ok());
        let value = session.get_user(*user).await?;
        on_found(
            name,
            ui::User {
                enabled: value.enabled,
                name: String::from_utf8_lossy(value.common_name.as_slice()).to_string().into(),
            },
        );
    }
    Ok(())
}

async fn set_enabled(
    backend: Rc<PeekCell<Backend>>,
    device_idx: usize,
    user_idx: usize,
    enabled: bool,
) -> Result<(), AppError> {
    let user = backend.peek(|backend| {
        let user_list = backend.get_user_list(device_idx)?;
        user_list.get(user_idx).ok_or(AppError::InternalError).cloned()
    })?;
    let session = backend.peek_mut(|backend| backend.get_user_session(device_idx))?;
    session.set_enabled(user, enabled).await
}

async fn set_name(
    backend: Rc<PeekCell<Backend>>,
    device_idx: usize,
    user_idx: usize,
    name: String,
) -> Result<(), AppError> {
    let user = backend.peek(|backend| {
        let user_list = backend.get_user_list(device_idx)?;
        user_list.get(user_idx).ok_or(AppError::InternalError).cloned()
    })?;
    let session = backend.peek_mut(|backend| backend.get_user_session(device_idx))?;
    session.set_name(user, name.as_str()).await
}

async fn set_password(
    backend: Rc<PeekCell<Backend>>,
    device_idx: usize,
    user_idx: usize,
    password: String,
) -> Result<(), AppError> {
    let user = backend.peek(|backend| {
        let user_list = backend.get_user_list(device_idx)?;
        user_list.get(user_idx).ok_or(AppError::InternalError).cloned()
    })?;
    let session = backend.peek_mut(|backend| backend.get_user_session(device_idx))?;
    session.set_password(user, password.as_bytes()).await
}

fn set_login_status(frontend: &Frontend, device_idx: usize, status: ui::ExtendedStatus) {
    frontend.with(|window| {
        let user_editor_state = window.global::<ui::UserEditorState>();
        let login_statuses = user_editor_state.get_login_statuses();
        if device_idx < login_statuses.row_count() {
            login_statuses.set_row_data(device_idx, status);
        }
    });
}

fn clear_users(frontend: &Frontend, device_idx: usize) {
    frontend.with(|window| {
        let user_editor_state = window.global::<ui::UserEditorState>();
        let user_lists = user_editor_state.get_user_lists();
        if device_idx < user_lists.row_count() {
            user_lists.set_row_data(device_idx, ui::UserList::empty());
        }
    });
}

fn push_user(frontend: &Frontend, device_idx: usize, name: String, value: ui::User) {
    frontend.with(|window| {
        let user_editor_state = window.global::<ui::UserEditorState>();
        let user_lists = user_editor_state.get_user_lists();
        if let Some(user_list) = user_lists.row_data(device_idx) {
            as_vec_model(&user_list.names).push(name.into());
            as_vec_model(&user_list.values).push(value);
            as_vec_model(&user_list.statuses).push(ui::ExtendedStatus::success());
        }
    });
}

fn get_user_value(frontend: &Frontend, device_idx: usize, user_idx: usize) -> Option<ui::User> {
    frontend
        .with(|window| {
            let user_editor_state = window.global::<ui::UserEditorState>();
            let user_lists = user_editor_state.get_user_lists();
            let user_list = user_lists.row_data(device_idx)?;
            user_list.values.row_data(user_idx)
        })
        .flatten()
}

fn set_user(frontend: &Frontend, device_idx: usize, user_idx: usize, result: Result<ui::User, AppError>) {
    frontend.with(|window| {
        let user_editor_state = window.global::<ui::UserEditorState>();
        let user_lists = user_editor_state.get_user_lists();
        if let Some(user_list) = user_lists.row_data(device_idx) {
            if let Ok(value) = result {
                if user_idx < user_list.values.row_count() {
                    user_list.values.set_row_data(user_idx, value);
                }
                if user_idx < user_list.values.row_count() {
                    user_list.statuses.set_row_data(user_idx, ui::ExtendedStatus::success());
                }
            } else {
                if user_idx < user_list.values.row_count() {
                    user_list.statuses.set_row_data(user_idx, ui::ExtendedStatus::from_result(result));
                }
            }
        }
    });
}

fn set_user_status(frontend: &Frontend, device_idx: usize, user_idx: usize, status: ui::ExtendedStatus) {
    frontend.with(|window| {
        let user_editor_state = window.global::<ui::UserEditorState>();
        let user_lists = user_editor_state.get_user_lists();
        if let Some(user_list) = user_lists.row_data(device_idx) {
            if user_idx < user_list.values.row_count() {
                user_list.statuses.set_row_data(user_idx, status);
            }
        }
    });
}
