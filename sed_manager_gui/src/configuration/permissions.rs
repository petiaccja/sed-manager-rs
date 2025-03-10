use sed_manager_gui_elements::ExtendedStatus;
use slint::{ComponentHandle as _, Model};
use std::rc::Rc;

use sed_manager::applications::{get_locking_sp, Error as AppError, PermissionEditSession};

use crate::backend::{get_object_name, Backend, EditorSession};
use crate::frontend::Frontend;
use crate::ui;
use crate::utility::{into_vec_model, PeekCell};

pub fn init(frontend: &Frontend, num_devices: usize) {
    frontend.with(|window| {
        let perm_editor_state = window.global::<ui::PermissionEditorState>();
        let initial_status = ui::ExtendedStatus::error("missing callback".into());
        perm_editor_state.set_login_statuses(into_vec_model(vec![initial_status; num_devices]));
        perm_editor_state.set_matrices(into_vec_model(vec![ui::PermissionMatrix::default(); num_devices]));
    });
}

pub fn clear(frontend: &Frontend) {
    init(frontend, 0);
}

pub fn set_callbacks(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    set_callback_login(backend.clone(), frontend.clone());
    set_callback_list(backend.clone(), frontend.clone());
    set_callback_fetch(backend.clone(), frontend.clone());
}

fn set_callback_login(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    frontend.clone().with(|window| {
        let perm_editor_state = window.global::<ui::PermissionEditorState>();

        perm_editor_state.on_login(move |device_idx, password| {
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
        let perm_editor_state = window.global::<ui::PermissionEditorState>();

        perm_editor_state.on_list(move |device_idx| {
            let frontend = frontend.clone();
            let backend = backend.clone();
            let device_idx = device_idx as usize;
            set_login_status(&frontend, device_idx, ui::ExtendedStatus::loading());
            let _ = slint::spawn_local(async move {
                let result = list(backend, device_idx).await;
                set_matrix(&frontend, device_idx, result);
            });
        });
    });
}

fn set_callback_fetch(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    frontend.clone().with(|window| {
        let perm_editor_state = window.global::<ui::PermissionEditorState>();

        perm_editor_state.on_fetch(move |device_idx, user_idx| {
            let frontend = frontend.clone();
            let backend = backend.clone();
            let device_idx = device_idx as usize;
            let user_idx = user_idx as usize;
            let _ = slint::spawn_local(async move {
                let result = fetch(backend, device_idx, user_idx).await;
                set_permission_list(&frontend, device_idx, user_idx, result);
            });
        });
    });
}

async fn login(backend: Rc<PeekCell<Backend>>, device_idx: usize, password: String) -> Result<(), AppError> {
    let tper = backend.peek_mut(|backend| backend.get_tper(device_idx))?;
    let session = PermissionEditSession::start(&tper, password.as_bytes()).await?;
    let editor_session = EditorSession::from(session);
    backend.peek_mut(|backend| backend.replace_session(device_idx, editor_session));
    Ok(())
}

async fn list(backend: Rc<PeekCell<Backend>>, device_idx: usize) -> Result<(Vec<String>, Vec<String>), AppError> {
    let session = backend.peek(|backend| backend.get_permission_session(device_idx))?;
    let discovery = backend.peek(|backend| backend.get_discovery(device_idx).cloned())?;
    let ssc = discovery.get_primary_ssc().ok_or(AppError::NoAvailableSSC)?;
    let locking_sp = get_locking_sp(ssc.feature_code()).ok();
    let users = session.list_users().await?;
    let ranges = session.list_ranges().await?;
    backend.peek_mut(|backend| backend.set_permission_matrix(device_idx, (users.clone(), ranges.clone())))?;
    let user_names = users
        .iter()
        .map(|uid| get_object_name(Some(&discovery), uid.as_uid(), locking_sp.clone()))
        .collect();
    let range_names = ranges
        .iter()
        .map(|uid| get_object_name(Some(&discovery), uid.as_uid(), locking_sp.clone()))
        .collect();
    Ok((user_names, range_names))
}

async fn fetch(
    backend: Rc<PeekCell<Backend>>,
    device_idx: usize,
    user_idx: usize,
) -> Result<ui::PermissionList, AppError> {
    let session = backend.peek(|backend| backend.get_permission_session(device_idx))?;
    let (users, ranges) = backend.peek(|backend| {
        backend
            .get_permission_matrix(device_idx)
            .map(|(users, ranges)| (Vec::from_iter(users.iter().cloned()), Vec::from_iter(ranges.iter().cloned())))
    })?;
    let user = users.get(user_idx).ok_or(AppError::InternalError)?;
    let unshadow_mbr = session.get_mbr_permission(*user).await?;
    let mut read_unlock = Vec::new();
    let mut write_unlock = Vec::new();
    for range in &ranges {
        read_unlock.push(session.get_read_permission(*user, *range).await?);
        write_unlock.push(session.get_write_permission(*user, *range).await?);
    }
    let mbr_status = ExtendedStatus::success();
    let range_statuses = core::iter::repeat_n(mbr_status.clone(), ranges.len()).collect();
    Ok(ui::PermissionList::new(unshadow_mbr, mbr_status, read_unlock, write_unlock, range_statuses))
}

fn set_login_status(frontend: &Frontend, device_idx: usize, status: ui::ExtendedStatus) {
    frontend.with(|window| {
        let perm_editor_state = window.global::<ui::PermissionEditorState>();
        let login_statuses = perm_editor_state.get_login_statuses();
        if device_idx < login_statuses.row_count() {
            login_statuses.set_row_data(device_idx, status);
        }
    });
}

fn set_matrix(frontend: &Frontend, device_idx: usize, matrix: Result<(Vec<String>, Vec<String>), AppError>) {
    frontend.with(|window| {
        let perm_editor_state = window.global::<ui::PermissionEditorState>();
        match matrix {
            Ok((users, ranges)) => {
                let default_permission_list = ui::PermissionList::blank(ranges.len());
                let permission_lists: Vec<_> = core::iter::repeat_n(default_permission_list, users.len()).collect();
                let matrix = ui::PermissionMatrix::new(users, ranges, permission_lists);
                let matrices = perm_editor_state.get_matrices();
                if device_idx < matrices.row_count() {
                    matrices.set_row_data(device_idx, matrix);
                }
                set_login_status(frontend, device_idx, ui::ExtendedStatus::success());
            }
            Err(error) => {
                set_login_status(frontend, device_idx, error.into());
            }
        }
    });
}

fn set_permission_list(
    frontend: &Frontend,
    device_idx: usize,
    user_idx: usize,
    perm_list: Result<ui::PermissionList, AppError>,
) {
    match perm_list {
        Ok(perm_list) => {
            frontend.with(|window| {
                let perm_editor_state = window.global::<ui::PermissionEditorState>();
                if let Some(matrix) = perm_editor_state.get_matrices().row_data(device_idx) {
                    if user_idx < matrix.permission_lists.row_count() {
                        matrix.permission_lists.set_row_data(user_idx, perm_list);
                    }
                }
            });
        }
        Err(error) => set_login_status(frontend, device_idx, error.into()),
    }
}
