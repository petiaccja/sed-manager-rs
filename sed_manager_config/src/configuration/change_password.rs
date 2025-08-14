//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::rc::Rc;

use sed_manager::spec;
use sed_manager::spec::objects::{Authority, CPIN};
use sed_manager_config_ui::ExtendedStatus;
use slint::{ComponentHandle as _, Model, SharedString};

use sed_manager::applications::{self, get_admin_sp, Error as AppError};

use crate::backend::{get_object_name, Backend, EditorSession};
use crate::frontend::Frontend;
use crate::ui;
use crate::utility::{into_vec_model, PeekCell};

pub fn init(frontend: &Frontend, num_devices: usize) {
    frontend.with(|window| {
        let change_pw_state = window.global::<ui::ChangePasswordState>();
        let status = ui::ExtendedStatus::error("missing callback".into());
        change_pw_state.set_statuses(into_vec_model(vec![status; num_devices]));
        change_pw_state.set_users(into_vec_model(vec![into_vec_model(Vec::<SharedString>::new()); num_devices]));
    });
}

pub fn clear(frontend: &Frontend) {
    init(frontend, 0);
}

pub fn set_callbacks(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    set_callback_list(backend.clone(), frontend.clone());
    set_callback_change_password(backend.clone(), frontend.clone());
}

fn set_callback_list(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    frontend.clone().with(|window| {
        let change_pw_state = window.global::<ui::ChangePasswordState>();

        change_pw_state.on_list(move |device_idx| {
            let frontend = frontend.clone();
            let backend = backend.clone();
            let device_idx = device_idx as usize;
            set_status(&frontend, device_idx, ui::ExtendedStatus::loading());
            let _ = slint::spawn_local(async move {
                let result = list(backend, device_idx).await;
                set_users(&frontend, device_idx, result);
            });
        });
    });
}

fn set_callback_change_password(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    frontend.clone().with(|window| {
        let change_pw_state = window.global::<ui::ChangePasswordState>();

        change_pw_state.on_change_password(move |device_idx, user_idx, password, new_password| {
            let frontend = frontend.clone();
            let backend = backend.clone();
            let device_idx = device_idx as usize;
            let user_idx = user_idx as usize;
            set_status(&frontend, device_idx, ui::ExtendedStatus::loading());
            let _ = slint::spawn_local(async move {
                let result = change_password(backend, device_idx, user_idx, password.into(), new_password.into()).await;
                set_status(&frontend, device_idx, ui::ExtendedStatus::from_result(result));
            });
        });
    });
}

async fn list(backend: Rc<PeekCell<Backend>>, device_idx: usize) -> Result<Vec<String>, AppError> {
    let tper = backend.peek_mut(|backend| backend.get_tper(device_idx))?;
    let discovery = tper.discover().await?;
    let ssc = discovery.get_primary_ssc();
    let admin_sp = ssc.map(|ssc| get_admin_sp(ssc.feature_code()).ok()).flatten();
    let password_auths = applications::list_password_authorities(&*tper).await?;

    let mut names = Vec::new();
    for (sp, auth) in password_auths.iter() {
        let sp_name = get_object_name(Some(&discovery), sp.as_uid(), admin_sp);
        let auth_name = get_object_name(Some(&discovery), auth.as_uid(), Some(*sp));
        names.push(format!("{sp_name}::{auth_name}"));
    }

    let session = backend.peek_mut(|backend| {
        backend.replace_session(device_idx, EditorSession::ChangePassword { users: password_auths })
    });
    if let Some(session) = session {
        let _ = session.end().await;
    }

    Ok(names)
}

async fn change_password(
    backend: Rc<PeekCell<Backend>>,
    device_idx: usize,
    user_idx: usize,
    password: String,
    new_password: String,
) -> Result<(), AppError> {
    let (sp, auth) = backend
        .peek_mut(|backend| {
            let session = backend.get_session(device_idx)?;
            match session {
                EditorSession::ChangePassword { users } => users.get(user_idx).cloned(),
                _ => None,
            }
        })
        .ok_or(AppError::InternalError)?;

    let tper = backend.peek_mut(|backend| backend.get_tper(device_idx))?;
    let session = tper.start_session(sp, Some(auth), Some(password.as_bytes())).await?;
    session
        .with(async |session| {
            let credential = if let Some(idx) = spec::opal::locking::authority::USER.index_of(auth) {
                // Unfortunately, User#N authorities don't have an ACE to query their own C_PIN credential.
                spec::opal::locking::c_pin::USER.nth(idx).ok_or(AppError::InternalError)?
            } else {
                session.get(auth.as_uid(), Authority::CREDENTIAL).await?
            };
            session.set(credential.as_uid(), CPIN::PIN, new_password.as_bytes()).await?;
            Ok(())
        })
        .await
}

fn set_users(frontend: &Frontend, device_idx: usize, users: Result<Vec<String>, AppError>) {
    frontend.with(|window| {
        let change_pw_state = window.global::<ui::ChangePasswordState>();
        let users_model = change_pw_state.get_users();
        match users {
            Ok(users) => {
                if device_idx < users_model.row_count() {
                    let users = users.into_iter().map(|s| s.into()).collect();
                    users_model.set_row_data(device_idx, into_vec_model(users));
                }
                set_status(frontend, device_idx, ExtendedStatus::success());
            }
            Err(error) => {
                set_status(frontend, device_idx, ExtendedStatus::from_result(Result::<(), _>::Err(error)));
            }
        }
    });
}

fn set_status(frontend: &Frontend, device_idx: usize, status: ui::ExtendedStatus) {
    frontend.with(|window| {
        let change_pw_state = window.global::<ui::ChangePasswordState>();
        let statuses = change_pw_state.get_statuses();
        if device_idx < statuses.row_count() {
            statuses.set_row_data(device_idx, status);
        }
    });
}
