use std::rc::Rc;

use sed_manager::spec::column_types::{CredentialRef, LockingRangeRef, MediaKeyRef};
use sed_manager::spec::{self, table_id};
use sed_manager::tper::Session;
use slint::{ComponentHandle as _, Model as _};

use sed_manager::applications::{get_locking_admins, get_locking_sp, Error as AppError};
use sed_manager::rpc::Error as RPCError;

use crate::backend::{get_object_name, Backend};
use crate::frontend::Frontend;
use crate::ui;
use crate::utility::{as_vec_model, into_vec_model, PeekCell};

pub fn init(frontend: &Frontend, num_devices: usize) {
    frontend.with(|window| {
        let range_editor_state = window.global::<ui::RangeEditorState>();
        let initial_status = ui::ExtendedStatus::error("missing callback".into());
        range_editor_state.set_login_statuses(into_vec_model(vec![initial_status; num_devices]));
        range_editor_state.set_range_lists(into_vec_model(vec![ui::RangeList::empty(); num_devices]));
    });
}

pub fn clear(frontend: &Frontend) {
    init(frontend, 0);
}

pub fn set_callbacks(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    set_callback_login(backend.clone(), frontend.clone());
    set_callback_list(backend.clone(), frontend.clone());
    set_callback_set_value(backend.clone(), frontend.clone());
    set_callback_erase(backend.clone(), frontend.clone());
}

fn set_callback_login(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    frontend.clone().with(|window| {
        let range_editor_state = window.global::<ui::RangeEditorState>();

        range_editor_state.on_login(move |device_idx, password| {
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
        let range_editor_state = window.global::<ui::RangeEditorState>();

        range_editor_state.on_list(move |device_idx| {
            let frontend = frontend.clone();
            let backend = backend.clone();
            let device_idx = device_idx as usize;
            clear_ranges(&frontend, device_idx);
            set_login_status(&frontend, device_idx, ui::ExtendedStatus::loading());
            let _ = slint::spawn_local(async move {
                let frontend_ = frontend.clone();
                let on_found = move |name, value| {
                    push_range(&frontend, device_idx, name, value);
                };
                let result = list(backend, device_idx, on_found).await;
                set_login_status(&frontend_, device_idx, ui::ExtendedStatus::from_result(result));
            });
        });
    });
}

fn set_callback_set_value(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    frontend.clone().with(|window| {
        let range_editor_state = window.global::<ui::RangeEditorState>();

        range_editor_state.on_set_value(move |device_idx, range_idx, value| {
            let frontend = frontend.clone();
            let backend = backend.clone();
            let device_idx = device_idx as usize;
            let range_idx = range_idx as usize;
            set_range_status(&frontend, device_idx, range_idx, ui::ExtendedStatus::loading());
            let _ = slint::spawn_local(async move {
                let result = set_value(backend, device_idx, range_idx, value.clone()).await.map(|_| value);
                set_range(&frontend, device_idx, range_idx, result);
            });
        });
    });
}

fn set_callback_erase(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    frontend.clone().with(|window| {
        let range_editor_state = window.global::<ui::RangeEditorState>();

        range_editor_state.on_erase(move |device_idx, range_idx| {
            let frontend = frontend.clone();
            let backend = backend.clone();
            let device_idx = device_idx as usize;
            let range_idx = range_idx as usize;
            set_range_status(&frontend, device_idx, range_idx, ui::ExtendedStatus::loading());
            let _ = slint::spawn_local(async move {
                let result = erase(backend, device_idx, range_idx).await;
                set_range_status(&frontend, device_idx, range_idx, ui::ExtendedStatus::from_result(result));
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
    on_found: impl Fn(String, ui::LockingRange),
) -> Result<(), AppError> {
    let session = backend.peek(|backend| backend.get_session(device_idx)).ok_or(RPCError::Unspecified)?;
    let discovery = backend.peek(|backend| backend.get_discovery(device_idx).cloned())?;
    let ssc = discovery.get_primary_ssc().ok_or(AppError::NoAvailableSSC)?;
    let locking_sp = get_locking_sp(ssc.feature_code());
    let ranges: Vec<_> = session
        .next(table_id::LOCKING, None, None)
        .await?
        .into_iter()
        .filter_map(|uid| LockingRangeRef::try_from(uid).ok())
        .collect();
    backend.peek_mut(|backend| backend.set_range_list(device_idx, ranges.clone()));
    for range in ranges.iter() {
        let name = get_object_name(Some(&discovery), range.as_uid(), locking_sp.clone().ok());
        if let Ok(range) = helpers::get_range_properties(&session, *range).await {
            on_found(name, range);
        }
    }
    Ok(())
}

async fn set_value(
    backend: Rc<PeekCell<Backend>>,
    device_idx: usize,
    range_idx: usize,
    value: ui::LockingRange,
) -> Result<(), AppError> {
    let range = backend.peek(|backend| {
        let user_list = backend.get_range_list(device_idx).ok_or(RPCError::Unspecified)?;
        user_list.get(range_idx).ok_or(RPCError::Unspecified).cloned()
    })?;
    let session = backend.peek_mut(|backend| backend.get_session(device_idx)).ok_or(RPCError::Unspecified)?;
    helpers::set_range_properties(&session, range, value).await?;
    Ok(())
}

async fn erase(backend: Rc<PeekCell<Backend>>, device_idx: usize, range_idx: usize) -> Result<(), AppError> {
    let range = backend.peek(|backend| {
        let user_list = backend.get_range_list(device_idx).ok_or(RPCError::Unspecified)?;
        user_list.get(range_idx).ok_or(RPCError::Unspecified).cloned()
    })?;
    let session = backend.peek_mut(|backend| backend.get_session(device_idx)).ok_or(RPCError::Unspecified)?;
    helpers::erase_locking_range(&session, range).await?;
    Ok(())
}

fn set_login_status(frontend: &Frontend, device_idx: usize, status: ui::ExtendedStatus) {
    frontend.with(|window| {
        let range_editor_state = window.global::<ui::RangeEditorState>();
        let login_statuses = range_editor_state.get_login_statuses();
        if device_idx < login_statuses.row_count() {
            login_statuses.set_row_data(device_idx, status);
        }
    });
}

fn clear_ranges(frontend: &Frontend, device_idx: usize) {
    frontend.with(|window| {
        let range_editor_state = window.global::<ui::RangeEditorState>();
        let range_lists = range_editor_state.get_range_lists();
        if device_idx < range_lists.row_count() {
            range_lists.set_row_data(device_idx, ui::RangeList::empty());
        }
    });
}

fn push_range(frontend: &Frontend, device_idx: usize, name: String, value: ui::LockingRange) {
    frontend.with(|window| {
        let range_editor_state = window.global::<ui::RangeEditorState>();
        let range_lists = range_editor_state.get_range_lists();
        if let Some(range_list) = range_lists.row_data(device_idx) {
            as_vec_model(&range_list.names).push(name.into());
            as_vec_model(&range_list.values).push(value);
            as_vec_model(&range_list.statuses).push(ui::ExtendedStatus::success());
            range_lists.set_row_data(device_idx, range_list);
        }
    });
}

fn set_range(frontend: &Frontend, device_idx: usize, range_idx: usize, result: Result<ui::LockingRange, AppError>) {
    frontend.with(|window| {
        let range_editor_state = window.global::<ui::RangeEditorState>();
        let range_lists = range_editor_state.get_range_lists();
        if let Some(range_list) = range_lists.row_data(device_idx) {
            if let Ok(value) = result {
                if range_idx < range_list.values.row_count() {
                    range_list.values.set_row_data(range_idx, value);
                }
                if range_idx < range_list.values.row_count() {
                    range_list.statuses.set_row_data(range_idx, ui::ExtendedStatus::success());
                }
            } else {
                if range_idx < range_list.values.row_count() {
                    range_list.statuses.set_row_data(range_idx, ui::ExtendedStatus::from_result(result));
                }
            }
        }
    });
}

fn set_range_status(frontend: &Frontend, device_idx: usize, range_idx: usize, status: ui::ExtendedStatus) {
    frontend.with(|window| {
        let range_editor_state = window.global::<ui::RangeEditorState>();
        let range_lists = range_editor_state.get_range_lists();
        if let Some(range_list) = range_lists.row_data(device_idx) {
            if range_idx < range_list.values.row_count() {
                range_list.statuses.set_row_data(range_idx, status);
            }
        }
    });
}

mod helpers {
    use sed_manager::spec::{column_types::LockingRangeRef, objects::LockingRange};

    use super::*;

    pub async fn get_range_properties(session: &Session, range: LockingRangeRef) -> Result<ui::LockingRange, RPCError> {
        let columns = LockingRange::RANGE_START..=LockingRange::WRITE_LOCKED;
        let (start_lba, length_lba, read_lock_enabled, write_lock_enabled, read_locked, write_locked) =
            session.get_multiple::<(u64, u64, bool, bool, bool, bool)>(range.as_uid(), columns).await?;

        Ok(ui::LockingRange {
            start_lba: start_lba as i32,
            end_lba: (start_lba + length_lba) as i32,
            read_lock_enabled,
            write_lock_enabled,
            read_locked,
            write_locked,
        })
    }

    pub async fn set_range_properties(
        session: &Session,
        range: LockingRangeRef,
        value: ui::LockingRange,
    ) -> Result<(), RPCError> {
        if range != spec::opal::locking::locking::GLOBAL_RANGE {
            let length_lba = value.end_lba - value.start_lba;
            let values = (
                value.start_lba as u64,
                length_lba as u64,
                value.read_lock_enabled,
                value.write_lock_enabled,
                value.read_locked,
                value.write_locked,
            );
            let columns: [u16; 6] = core::array::from_fn(|i| LockingRange::RANGE_START + (i as u16));
            session.set_multiple(range.as_uid(), columns, values).await
        } else {
            let values = (value.read_lock_enabled, value.write_lock_enabled, value.read_locked, value.write_locked);
            let columns: [u16; 4] = core::array::from_fn(|i| LockingRange::READ_LOCK_ENABLED + (i as u16));
            session.set_multiple(range.as_uid(), columns, values).await
        }
    }

    pub async fn erase_locking_range(session: &Session, range: LockingRangeRef) -> Result<(), RPCError> {
        let active_key_id: MediaKeyRef = session.get(range.as_uid(), LockingRange::ACTIVE_KEY).await?;
        session.gen_key(CredentialRef::new_other(active_key_id), None, None).await
    }
}
