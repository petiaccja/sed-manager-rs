use std::rc::Rc;

use slint::{ComponentHandle as _, Model};

use sed_manager::applications::{get_locking_admins, get_locking_sp, Error as AppError};

use crate::backend::Backend;
use crate::frontend::Frontend;
use crate::ui;
use crate::utility::{into_vec_model, PeekCell};

pub fn init(frontend: &Frontend, num_devices: usize) {
    frontend.with(|window| {
        let mbr_editor_state = window.global::<ui::MBREditorState>();
        let initial_status = ui::ExtendedStatus::error("missing callback".into());
        mbr_editor_state.set_login_statuses(into_vec_model(vec![initial_status; num_devices]));
        mbr_editor_state.set_mbr_control(into_vec_model(vec![ui::MBRControl::default(); num_devices]));
    });
}

pub fn clear(frontend: &Frontend) {
    init(frontend, 0);
}

pub fn set_callbacks(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    set_callback_login(backend.clone(), frontend.clone());
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

fn set_login_status(frontend: &Frontend, device_idx: usize, status: ui::ExtendedStatus) {
    frontend.with(|window| {
        let mbr_editor_state = window.global::<ui::MBREditorState>();
        let login_statuses = mbr_editor_state.get_login_statuses();
        if device_idx < login_statuses.row_count() {
            login_statuses.set_row_data(device_idx, status);
        }
    });
}
