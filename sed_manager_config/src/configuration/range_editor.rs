//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::rc::Rc;

use sed_manager::messaging::discovery::GeometryDescriptor;
use sed_manager::spec::objects::LockingRange;
use slint::{ComponentHandle as _, Model as _};

use sed_manager::applications::{get_locking_sp, Error as AppError, RangeEditSession};

use crate::backend::{get_object_name, Backend, EditorSession};
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
                let result = set_value(backend, device_idx, range_idx, value).await;
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
    let session = RangeEditSession::start(&tper, password.as_bytes()).await?;
    let editor_session = EditorSession::from(session);
    backend.peek_mut(|backend| backend.replace_session(device_idx, editor_session));
    Ok(())
}

async fn list(
    backend: Rc<PeekCell<Backend>>,
    device_idx: usize,
    on_found: impl Fn(String, ui::LockingRange),
) -> Result<(), AppError> {
    let session = backend.peek(|backend| backend.get_range_session(device_idx))?;
    let discovery = backend.peek(|backend| backend.get_discovery(device_idx).cloned())?;
    let ssc = discovery.get_primary_ssc().ok_or(AppError::NoAvailableSSC)?;
    let locking_sp = get_locking_sp(ssc.feature_code());
    let ranges: Vec<_> = session.list_ranges().await?;
    backend.peek_mut(|backend| backend.set_range_list(device_idx, ranges.clone()))?;
    for range in ranges.iter() {
        let name = get_object_name(Some(&discovery), range.as_uid(), locking_sp.clone().ok());
        let value = session.get_range(*range).await?;
        on_found(
            name,
            ui::LockingRange {
                start_lba: value.range_start as i64,
                end_lba: (value.range_start + value.range_length) as i64,
                read_lock_enabled: value.read_lock_enabled,
                write_lock_enabled: value.write_lock_enabled,
                read_locked: value.read_locked,
                write_locked: value.write_locked,
            },
        );
    }
    Ok(())
}

async fn set_value(
    backend: Rc<PeekCell<Backend>>,
    device_idx: usize,
    range_idx: usize,
    value: ui::LockingRange,
) -> Result<ui::LockingRange, AppError> {
    let range = backend.peek(|backend| {
        let range_list = backend.get_range_list(device_idx)?;
        range_list.get(range_idx).ok_or(AppError::InternalError).cloned()
    })?;
    let geometry = backend.peek(|backend| {
        let default =
            GeometryDescriptor { align: false, logical_block_size: 1, alignment_granularity: 1, lowest_aligned_lba: 0 };
        backend
            .get_discovery(device_idx)
            .ok()
            .map(|discovery| discovery.get::<GeometryDescriptor>())
            .flatten()
            .cloned()
            .unwrap_or(default)
    });
    let value = align_locking_range(value, &geometry);
    let session = backend.peek_mut(|backend| backend.get_range_session(device_idx))?;
    let lr = LockingRange {
        uid: range,
        range_start: value.start_lba as u64,
        range_length: (value.end_lba - value.start_lba) as u64,
        read_lock_enabled: value.read_lock_enabled,
        write_lock_enabled: value.write_lock_enabled,
        read_locked: value.read_locked,
        write_locked: value.write_locked,
        ..Default::default()
    };
    session.set_range(&lr).await.map(|_| value)
}

async fn erase(backend: Rc<PeekCell<Backend>>, device_idx: usize, range_idx: usize) -> Result<(), AppError> {
    let range = backend.peek(|backend| {
        let range_list = backend.get_range_list(device_idx)?;
        range_list.get(range_idx).ok_or(AppError::InternalError).cloned()
    })?;
    let session = backend.peek_mut(|backend| backend.get_range_session(device_idx))?;
    session.erase_range(range).await
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

fn align_lba(lba: u64, block_alignment: u64, first_aligned_block: u64) -> u64 {
    let block_alignment = std::cmp::max(1, block_alignment); // Zero means no alignment, clamp to 1..infinity.
    let clamped_lba = std::cmp::max(lba, first_aligned_block);
    let base_lba = clamped_lba - first_aligned_block;
    let aligned_base_lba = base_lba / block_alignment * block_alignment;
    first_aligned_block + aligned_base_lba
}

fn align_locking_range(range: ui::LockingRange, geometry: &GeometryDescriptor) -> ui::LockingRange {
    let start_lba = align_lba(
        std::cmp::max(0, range.start_lba) as u64,
        geometry.alignment_granularity,
        geometry.lowest_aligned_lba,
    ) as i64;
    let end_lba = align_lba(
        std::cmp::max(start_lba, std::cmp::max(0, range.end_lba)) as u64,
        geometry.alignment_granularity,
        geometry.lowest_aligned_lba,
    ) as i64;
    ui::LockingRange { start_lba, end_lba, ..range }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn align_lba_zero_alignment() {
        assert_eq!(align_lba(11, 0, 7), 11);
    }

    #[test]
    fn align_lba_offset_aligned() {
        assert_eq!(align_lba(11, 4, 7), 11);
    }

    #[test]
    fn align_lba_offset_misaligned() {
        assert_eq!(align_lba(13, 4, 7), 11);
    }

    #[test]
    fn align_lba_nooffset_aligned() {
        assert_eq!(align_lba(12, 4, 0), 12);
    }

    #[test]
    fn align_lba_nooffset_misaligned() {
        assert_eq!(align_lba(13, 4, 0), 12);
    }

    #[test]
    fn align_locking_range_with_geom() {
        let geometry = GeometryDescriptor {
            align: true,
            logical_block_size: 512,
            alignment_granularity: 8,
            lowest_aligned_lba: 4,
        };
        let range = ui::LockingRange { start_lba: 13, end_lba: 23, ..Default::default() };
        let aligned = align_locking_range(range, &geometry);
        assert_eq!(aligned, ui::LockingRange { start_lba: 12, end_lba: 20, ..Default::default() });
    }
}
