use std::rc::Rc;

use sed_manager::spec::column_types::{AuthorityRef, CPINRef, Name};
use sed_manager::spec::objects::{Authority, CPIN};
use sed_manager::spec::{self, table_id};
use sed_manager::tper::Session;
use slint::{ComponentHandle as _, Model};

use sed_manager::applications::{get_locking_admins, get_locking_sp, Error as AppError};
use sed_manager::rpc::Error as RPCError;

use crate::backend::{get_object_name, Backend};
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
    let discovery = backend.peek_mut(|backend| backend.get_discovery(device_idx).cloned())?;
    let ssc = discovery.get_primary_ssc().ok_or(AppError::IncompatibleSSC)?;
    let locking_sp = get_locking_sp(ssc.feature_code())?;
    let admin1 = get_locking_admins(ssc.feature_code())?.nth(1).unwrap();
    let session = tper.start_session(locking_sp, Some(admin1), Some(password.as_bytes())).await?;
    backend.peek_mut(|backend| backend.replace_session(device_idx, session));
    Ok(())
}

async fn list(
    backend: Rc<PeekCell<Backend>>,
    device_idx: usize,
    on_found: impl Fn(String, ui::User),
) -> Result<(), AppError> {
    let session = backend.peek(|backend| backend.get_session(device_idx)).ok_or(RPCError::Unspecified)?;
    let discovery = backend.peek(|backend| backend.get_discovery(device_idx).cloned())?;
    let ssc = discovery.get_primary_ssc().ok_or(AppError::NoAvailableSSC)?;
    let locking_sp = get_locking_sp(ssc.feature_code());
    let authorities = session
        .next(table_id::AUTHORITY, None, None)
        .await?
        .into_iter()
        .filter_map(|uid| AuthorityRef::try_from(uid).ok())
        .collect();
    let authorities = helpers::retain_configurable_authorities(&session, authorities).await;
    backend.peek_mut(|backend| backend.set_user_list(device_idx, authorities.clone()));
    for authority in authorities.iter() {
        let name = get_object_name(Some(&discovery), authority.as_uid(), locking_sp.clone().ok());
        if let Ok(user) = helpers::get_authority_properties(&session, *authority).await {
            on_found(name, user);
        }
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
        let user_list = backend.get_user_list(device_idx).ok_or(RPCError::Unspecified)?;
        user_list.get(user_idx).ok_or(RPCError::Unspecified).cloned()
    })?;
    let session = backend.peek_mut(|backend| backend.get_session(device_idx)).ok_or(RPCError::Unspecified)?;
    session.set(user.as_uid(), Authority::ENABLED, enabled).await?;
    Ok(())
}

async fn set_name(
    backend: Rc<PeekCell<Backend>>,
    device_idx: usize,
    user_idx: usize,
    name: String,
) -> Result<(), AppError> {
    let user = backend.peek(|backend| {
        let user_list = backend.get_user_list(device_idx).ok_or(RPCError::Unspecified)?;
        user_list.get(user_idx).ok_or(RPCError::Unspecified).cloned()
    })?;
    let session = backend.peek_mut(|backend| backend.get_session(device_idx)).ok_or(RPCError::Unspecified)?;
    session.set(user.as_uid(), Authority::COMMON_NAME, name.as_bytes()).await?;
    Ok(())
}

async fn set_password(
    backend: Rc<PeekCell<Backend>>,
    device_idx: usize,
    user_idx: usize,
    password: String,
) -> Result<(), AppError> {
    let user = backend.peek(|backend| {
        let user_list = backend.get_user_list(device_idx).ok_or(RPCError::Unspecified)?;
        user_list.get(user_idx).ok_or(RPCError::Unspecified).cloned()
    })?;
    let session = backend.peek_mut(|backend| backend.get_session(device_idx)).ok_or(RPCError::Unspecified)?;
    let credential: CPINRef = session.get(user.as_uid(), Authority::CREDENTIAL).await?;
    session.set(credential.as_uid(), CPIN::PIN, password.as_bytes()).await?;
    Ok(())
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

mod helpers {
    use super::*;

    pub async fn retain_configurable_authorities(
        session: &Session,
        authorities: Vec<AuthorityRef>,
    ) -> Vec<AuthorityRef> {
        let mut non_class = Vec::new();
        for authority in authorities {
            let is_not_just_anybody = authority != spec::core::authority::ANYBODY;
            let is_not_class = Ok(false) == session.get(authority.as_uid(), Authority::IS_CLASS).await;
            if is_not_just_anybody && is_not_class {
                non_class.push(authority);
            }
        }
        return non_class;
    }

    pub async fn get_authority_properties(session: &Session, authority: AuthorityRef) -> Result<ui::User, RPCError> {
        let name: Name = session.get(authority.as_uid(), Authority::COMMON_NAME).await?;
        let enabled: bool = session.get(authority.as_uid(), Authority::ENABLED).await?;

        Ok(ui::User { name: String::try_from(name).unwrap_or("".into()).into(), enabled })
    }
}
