use std::rc::Rc;

use sed_manager::messaging::com_id::StackResetStatus;
use slint::{ComponentHandle as _, Model};

use sed_manager::applications::Error as AppError;
use sed_manager::rpc::Error as RPCError;

use crate::backend::Backend;
use crate::frontend::Frontend;
use crate::ui;
use crate::utility::{into_vec_model, PeekCell};

pub fn init(frontend: &Frontend, num_devices: usize) {
    frontend.with(|window| {
        let troubleshooting_state = window.global::<ui::TroubleshootingState>();
        let initial_status = ui::ExtendedStatus::error("missing callback".into());
        troubleshooting_state.set_statuses(into_vec_model(vec![initial_status; num_devices]));
    });
}

pub fn clear(frontend: &Frontend) {
    init(frontend, 0);
}

pub fn set_callbacks(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    set_stack_reset(backend.clone(), frontend.clone());
}

fn set_stack_reset(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    frontend.clone().with(|window| {
        let troubleshooting_state = window.global::<ui::TroubleshootingState>();

        troubleshooting_state.on_stack_reset(move |device_idx| {
            let frontend = frontend.clone();
            let backend = backend.clone();
            let device_idx = device_idx as usize;
            set_status(&frontend, device_idx, ui::ExtendedStatus::loading());
            let _ = slint::spawn_local(async move {
                let result = stack_reset(backend, device_idx).await;
                set_status(&frontend, device_idx, ui::ExtendedStatus::from_result(result));
            });
        });
    });
}

async fn stack_reset(backend: Rc<PeekCell<Backend>>, device_idx: usize) -> Result<(), AppError> {
    let tper = backend.peek_mut(|backend| backend.get_tper(device_idx))?;
    let result = tper.stack_reset(tper.com_id(), tper.com_id_ext()).await?;
    match result {
        StackResetStatus::Success => Ok(()),
        StackResetStatus::Failure => Err(RPCError::Unspecified.into()),
        StackResetStatus::Pending => Ok(()),
    }
}

fn set_status(frontend: &Frontend, device_idx: usize, status: ui::ExtendedStatus) {
    frontend.with(|window| {
        let troubleshooting_state = window.global::<ui::TroubleshootingState>();
        let statuses = troubleshooting_state.get_statuses();
        if device_idx < statuses.row_count() {
            statuses.set_row_data(device_idx, status);
        }
    });
}
