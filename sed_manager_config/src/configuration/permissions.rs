use sed_manager::spec;
use sed_manager::spec::column_types::{AuthorityRef, LockingRangeRef};
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
    set_callback_mbr_permission(backend.clone(), frontend.clone());
    set_callback_read_permission(backend.clone(), frontend.clone());
    set_callback_write_permission(backend.clone(), frontend.clone());
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
            set_matrix(&frontend, device_idx, Ok((vec![], vec![], false)));
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

fn set_callback_mbr_permission(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    frontend.clone().with(|window| {
        let perm_editor_state = window.global::<ui::PermissionEditorState>();

        perm_editor_state.on_set_mbr_permission(move |device_idx, user_idx, permitted| {
            let frontend = frontend.clone();
            let backend = backend.clone();
            let device_idx = device_idx as usize;
            let user_idx = user_idx as usize;
            set_mbr_update_progress(&frontend, device_idx, user_idx, ui::ExtendedStatus::loading());
            let _ = slint::spawn_local(async move {
                let result = set_mbr_permission(backend.clone(), device_idx, user_idx, permitted).await;
                set_mbr_update_result(&frontend, device_idx, user_idx, result);
                if is_class(backend, device_idx, user_idx) {
                    invalidate_users(&frontend, device_idx, user_idx);
                }
            });
        });
    });
}

fn set_callback_read_permission(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    frontend.clone().with(|window| {
        let perm_editor_state = window.global::<ui::PermissionEditorState>();

        perm_editor_state.on_set_read_permission(move |device_idx, user_idx, range_idx, permitted| {
            let frontend = frontend.clone();
            let backend = backend.clone();
            let device_idx = device_idx as usize;
            let user_idx = user_idx as usize;
            let range_idx = range_idx as usize;
            set_range_update_progress(&frontend, device_idx, user_idx, range_idx, ui::ExtendedStatus::loading());
            let _ = slint::spawn_local(async move {
                let result = set_read_permission(backend.clone(), device_idx, user_idx, range_idx, permitted).await;
                set_read_update_result(&frontend, device_idx, user_idx, range_idx, result);
                if is_class(backend, device_idx, user_idx) {
                    invalidate_users(&frontend, device_idx, user_idx);
                }
            });
        });
    });
}

fn set_callback_write_permission(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    frontend.clone().with(|window| {
        let perm_editor_state = window.global::<ui::PermissionEditorState>();

        perm_editor_state.on_set_write_permission(move |device_idx, user_idx, range_idx, permitted| {
            let frontend = frontend.clone();
            let backend = backend.clone();
            let device_idx = device_idx as usize;
            let user_idx = user_idx as usize;
            let range_idx = range_idx as usize;
            set_range_update_progress(&frontend, device_idx, user_idx, range_idx, ui::ExtendedStatus::loading());
            let _ = slint::spawn_local(async move {
                let result = set_write_permission(backend.clone(), device_idx, user_idx, range_idx, permitted).await;
                set_write_update_result(&frontend, device_idx, user_idx, range_idx, result);
                if is_class(backend, device_idx, user_idx) {
                    invalidate_users(&frontend, device_idx, user_idx);
                }
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

async fn list(backend: Rc<PeekCell<Backend>>, device_idx: usize) -> Result<(Vec<String>, Vec<String>, bool), AppError> {
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
    let mbr_supported = session.is_mbr_supported().await;
    Ok((user_names, range_names, mbr_supported))
}

fn get_cached_matrix(
    backend: &Backend,
    device_idx: usize,
) -> Result<(Vec<AuthorityRef>, Vec<LockingRangeRef>), AppError> {
    backend
        .get_permission_matrix(device_idx)
        .map(|(users, ranges)| (Vec::from_iter(users.iter().cloned()), Vec::from_iter(ranges.iter().cloned())))
}

async fn fetch(
    backend: Rc<PeekCell<Backend>>,
    device_idx: usize,
    user_idx: usize,
) -> Result<ui::PermissionList, AppError> {
    let session = backend.peek(|backend| backend.get_permission_session(device_idx))?;
    let (users, ranges) = backend.peek(|backend| get_cached_matrix(backend, device_idx))?;
    let user = users.get(user_idx).ok_or(AppError::InternalError)?;
    let unshadow_mbr = session.get_mbr_permission(*user).await?;
    let mut read_unlock = Vec::new();
    let mut write_unlock = Vec::new();
    for range in &ranges {
        read_unlock.push(session.get_read_permission(*user, *range).await?);
        write_unlock.push(session.get_write_permission(*user, *range).await?);
    }
    let mbr_status = ui::ExtendedStatus::success();
    let range_statuses = core::iter::repeat_n(mbr_status.clone(), ranges.len()).collect();
    Ok(ui::PermissionList::new(unshadow_mbr, mbr_status, read_unlock, write_unlock, range_statuses))
}

async fn set_mbr_permission(
    backend: Rc<PeekCell<Backend>>,
    device_idx: usize,
    user_idx: usize,
    permitted: bool,
) -> Result<bool, AppError> {
    let session = backend.peek(|backend| backend.get_permission_session(device_idx))?;
    let (users, _ranges) = backend.peek(|backend| get_cached_matrix(backend, device_idx))?;
    let user = users.get(user_idx).ok_or(AppError::InternalError)?;
    session.set_mbr_permission(*user, permitted).await?;
    Ok(session.get_mbr_permission(*user).await.unwrap_or(permitted))
}

async fn set_read_permission(
    backend: Rc<PeekCell<Backend>>,
    device_idx: usize,
    user_idx: usize,
    range_idx: usize,
    permitted: bool,
) -> Result<bool, AppError> {
    let session = backend.peek(|backend| backend.get_permission_session(device_idx))?;
    let (users, ranges) = backend.peek(|backend| get_cached_matrix(backend, device_idx))?;
    let user = users.get(user_idx).ok_or(AppError::InternalError)?;
    let range = ranges.get(range_idx).ok_or(AppError::InternalError)?;
    session.set_read_permission(*user, *range, permitted).await?;
    // Permission may also be affected by class permissions, better refresh.
    Ok(session.get_read_permission(*user, *range).await.unwrap_or(permitted))
}

async fn set_write_permission(
    backend: Rc<PeekCell<Backend>>,
    device_idx: usize,
    user_idx: usize,
    range_idx: usize,
    permitted: bool,
) -> Result<bool, AppError> {
    let session = backend.peek(|backend| backend.get_permission_session(device_idx))?;
    let (users, ranges) = backend.peek(|backend| get_cached_matrix(backend, device_idx))?;
    let user = users.get(user_idx).ok_or(AppError::InternalError)?;
    let range = ranges.get(range_idx).ok_or(AppError::InternalError)?;
    session.set_write_permission(*user, *range, permitted).await?;
    // Permission may also be affected by class permissions, better refresh.
    Ok(session.get_write_permission(*user, *range).await.unwrap_or(permitted))
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

fn set_user_status(frontend: &Frontend, device_idx: usize, user_idx: usize, status: ui::ExtendedStatus) {
    frontend.with(|window| {
        let perm_editor_state = window.global::<ui::PermissionEditorState>();
        let matrices = perm_editor_state.get_matrices();

        if let Some(matrix) = matrices.row_data(device_idx) {
            if user_idx < matrix.user_statuses.row_count() {
                matrix.user_statuses.set_row_data(user_idx, status);
            }
            matrices.set_row_data(device_idx, matrix);
        }
    });
}

fn set_matrix(frontend: &Frontend, device_idx: usize, matrix: Result<(Vec<String>, Vec<String>, bool), AppError>) {
    frontend.with(|window| {
        let perm_editor_state = window.global::<ui::PermissionEditorState>();
        match matrix {
            Ok((users, ranges, mbr_supported)) => {
                let default_permission_list = ui::PermissionList::blank(ranges.len());
                let permission_lists: Vec<_> = core::iter::repeat_n(default_permission_list, users.len()).collect();
                let user_statuses: Vec<_> = core::iter::repeat_n(ui::ExtendedStatus::loading(), users.len()).collect();
                let matrix = ui::PermissionMatrix::new(users, ranges, mbr_supported, user_statuses, permission_lists);
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
    if let Ok(perm_list) = perm_list {
        frontend.with(|window| {
            let perm_editor_state = window.global::<ui::PermissionEditorState>();
            if let Some(matrix) = perm_editor_state.get_matrices().row_data(device_idx) {
                if user_idx < matrix.permission_lists.row_count() {
                    matrix.permission_lists.set_row_data(user_idx, perm_list);
                }
            }
        });
        set_user_status(frontend, device_idx, user_idx, ui::ExtendedStatus::success());
    } else {
        set_user_status(frontend, device_idx, user_idx, ui::ExtendedStatus::from_result(perm_list));
    }
}

fn set_mbr_update_progress(frontend: &Frontend, device_idx: usize, user_idx: usize, status: ui::ExtendedStatus) {
    frontend.with(|window| {
        let perm_editor_state = window.global::<ui::PermissionEditorState>();
        if let Some(matrix) = perm_editor_state.get_matrices().row_data(device_idx) {
            if let Some(mut perm_list) = matrix.permission_lists.row_data(user_idx) {
                perm_list.unshadow_mbr_status = status;
                matrix.permission_lists.set_row_data(user_idx, perm_list);
            }
        }
    });
}

fn set_mbr_update_result(frontend: &Frontend, device_idx: usize, user_idx: usize, result: Result<bool, AppError>) {
    frontend.with(|window| {
        let perm_editor_state = window.global::<ui::PermissionEditorState>();
        if let Ok(permitted) = &result {
            if let Some(matrix) = perm_editor_state.get_matrices().row_data(device_idx) {
                if let Some(mut perm_list) = matrix.permission_lists.row_data(user_idx) {
                    perm_list.unshadow_mbr = *permitted;
                    matrix.permission_lists.set_row_data(user_idx, perm_list);
                }
            }
        }
    });
    set_mbr_update_progress(frontend, device_idx, user_idx, ui::ExtendedStatus::from_result(result));
}

fn set_range_update_progress(
    frontend: &Frontend,
    device_idx: usize,
    user_idx: usize,
    range_idx: usize,
    status: ui::ExtendedStatus,
) {
    frontend.with(|window| {
        let perm_editor_state = window.global::<ui::PermissionEditorState>();
        if let Some(matrix) = perm_editor_state.get_matrices().row_data(device_idx) {
            if let Some(perm_list) = matrix.permission_lists.row_data(user_idx) {
                if let Some(_range_status) = perm_list.range_statuses.row_data(range_idx) {
                    perm_list.range_statuses.set_row_data(range_idx, status);
                }
            }
        }
    });
}

fn set_read_update_result(
    frontend: &Frontend,
    device_idx: usize,
    user_idx: usize,
    range_idx: usize,
    result: Result<bool, AppError>,
) {
    frontend.with(|window| {
        let perm_editor_state = window.global::<ui::PermissionEditorState>();
        if let Ok(permitted) = &result {
            if let Some(matrix) = perm_editor_state.get_matrices().row_data(device_idx) {
                if let Some(perm_list) = matrix.permission_lists.row_data(user_idx) {
                    if let Some(_range_status) = perm_list.range_statuses.row_data(range_idx) {
                        perm_list.read_unlock.set_row_data(range_idx, *permitted);
                    }
                }
            }
        }
    });
    set_range_update_progress(frontend, device_idx, user_idx, range_idx, ui::ExtendedStatus::from_result(result));
}

fn set_write_update_result(
    frontend: &Frontend,
    device_idx: usize,
    user_idx: usize,
    range_idx: usize,
    result: Result<bool, AppError>,
) {
    frontend.with(|window| {
        let perm_editor_state = window.global::<ui::PermissionEditorState>();
        if let Ok(permitted) = &result {
            if let Some(matrix) = perm_editor_state.get_matrices().row_data(device_idx) {
                if let Some(perm_list) = matrix.permission_lists.row_data(user_idx) {
                    if let Some(_range_status) = perm_list.range_statuses.row_data(range_idx) {
                        perm_list.write_unlock.set_row_data(range_idx, *permitted);
                    }
                }
            }
        }
    });
    set_range_update_progress(frontend, device_idx, user_idx, range_idx, ui::ExtendedStatus::from_result(result));
}

fn is_class(backend: Rc<PeekCell<Backend>>, device_idx: usize, user_idx: usize) -> bool {
    let user = backend.peek(|backend| {
        backend
            .get_permission_matrix(device_idx)
            .map(|(users, _)| users.get(user_idx).cloned())
            .ok()
            .flatten()
    });
    // The UIDs are the same for all relevant SSCs.
    match user.unwrap_or(AuthorityRef::null()) {
        spec::opal::locking::authority::USERS => true,
        spec::opal::locking::authority::ADMINS => true,
        _ => false,
    }
}

fn invalidate_users(frontend: &Frontend, device_idx: usize, class_idx: usize) {
    frontend.with(|window| {
        let perm_editor_state = window.global::<ui::PermissionEditorState>();
        if let Some(matrix) = perm_editor_state.get_matrices().row_data(device_idx) {
            let num_users = matrix.user_statuses.row_count();
            for user_idx in 0..num_users {
                if user_idx != class_idx {
                    matrix.user_statuses.set_row_data(user_idx, ui::ExtendedStatus::loading());
                }
            }
        }
    });
}
