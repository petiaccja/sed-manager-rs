use std::rc::Rc;

use slint::{ComponentHandle as _, Model};

use sed_manager::applications::{self, get_admin_sp, get_locking_sp, Error as AppError};

use crate::backend::Backend;
use crate::frontend::Frontend;
use crate::ui;
use crate::utility::{into_vec_model, PeekCell};

pub fn init(frontend: &Frontend, num_devices: usize) {
    frontend.with(|window| {
        let single_step_state = window.global::<ui::SingleStepState>();
        let initial_status = ui::ExtendedStatus::error("missing callback".into());
        single_step_state.set_statuses(into_vec_model(vec![initial_status; num_devices]));
    });
}

pub fn clear(frontend: &Frontend) {
    init(frontend, 0);
}

pub fn set_callbacks(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    set_take_ownership(backend.clone(), frontend.clone());
    set_activate_locking(backend.clone(), frontend.clone());
    set_revert(backend.clone(), frontend.clone());
}

fn set_take_ownership(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    frontend.clone().with(|window| {
        let single_step_state = window.global::<ui::SingleStepState>();

        single_step_state.on_take_ownership(move |device_idx, password| {
            let frontend = frontend.clone();
            let backend = backend.clone();
            let device_idx = device_idx as usize;
            let password = String::from(password);
            set_status(&frontend, device_idx, ui::ExtendedStatus::loading());
            let _ = slint::spawn_local(async move {
                let result = take_ownership(backend, device_idx, password).await;
                set_status(&frontend, device_idx, ui::ExtendedStatus::from_result(result));
            });
        });
    });
}

fn set_activate_locking(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    frontend.clone().with(|window| {
        let single_step_state = window.global::<ui::SingleStepState>();

        single_step_state.on_activate_locking(move |device_idx, sid_password, locking_password| {
            let frontend = frontend.clone();
            let backend = backend.clone();
            let device_idx = device_idx as usize;
            let sid_password = String::from(sid_password);
            let locking_password = String::from(locking_password);
            set_status(&frontend, device_idx, ui::ExtendedStatus::loading());
            let _ = slint::spawn_local(async move {
                let result = activate_locking(backend, device_idx, sid_password, locking_password).await;
                set_status(&frontend, device_idx, ui::ExtendedStatus::from_result(result));
            });
        });
    });
}

fn set_revert(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    frontend.clone().with(|window| {
        let single_step_state = window.global::<ui::SingleStepState>();

        single_step_state.on_revert(move |device_idx, use_psid, password, revert_admin| {
            let frontend = frontend.clone();
            let backend = backend.clone();
            let device_idx = device_idx as usize;
            let password = String::from(password);
            set_status(&frontend, device_idx, ui::ExtendedStatus::loading());
            let _ = slint::spawn_local(async move {
                let result = revert(backend, device_idx, use_psid, password, revert_admin).await;
                set_status(&frontend, device_idx, ui::ExtendedStatus::from_result(result));
            });
        });
    });
}

async fn take_ownership(
    backend: Rc<PeekCell<Backend>>,
    device_idx: usize,
    new_password: String,
) -> Result<(), AppError> {
    let tper = backend.peek_mut(|backend| backend.get_tper(device_idx))?;
    applications::take_ownership(&*tper, new_password.as_bytes()).await
}

async fn activate_locking(
    backend: Rc<PeekCell<Backend>>,
    device_idx: usize,
    sid_password: String,
    new_locking_password: String,
) -> Result<(), AppError> {
    let tper = backend.peek_mut(|backend| backend.get_tper(device_idx))?;
    applications::activate_locking(&*&tper, sid_password.as_bytes(), Some(new_locking_password.as_bytes())).await
}

async fn revert(
    backend: Rc<PeekCell<Backend>>,
    device_idx: usize,
    use_psid: bool,
    password: String,
    revert_admin: bool,
) -> Result<(), AppError> {
    use sed_manager::spec::core::authority::SID;
    use sed_manager::spec::psid::admin::authority::PSID;

    let tper = backend.peek_mut(|backend| backend.get_tper(device_idx))?;
    let discovery = backend.peek_mut(|backend| backend.get_discovery(device_idx).cloned())?;
    let ssc = discovery.get_primary_ssc().ok_or(AppError::IncompatibleSSC)?;
    let admin_sp = get_admin_sp(ssc.feature_code())?;
    let locking_sp = get_locking_sp(ssc.feature_code())?;
    let authority = if use_psid { PSID } else { SID };
    let sp = if revert_admin { admin_sp } else { locking_sp };
    applications::revert(&*tper, authority, password.as_bytes(), sp).await
}

fn set_status(frontend: &Frontend, device_idx: usize, status: ui::ExtendedStatus) {
    frontend.with(|window| {
        let single_step_state = window.global::<ui::SingleStepState>();
        let statuses = single_step_state.get_statuses();
        if device_idx < statuses.row_count() {
            statuses.set_row_data(device_idx, status);
        }
    });
}
