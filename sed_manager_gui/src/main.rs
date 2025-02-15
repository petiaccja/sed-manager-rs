#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_state;
mod atomic_borrow;
mod generated;
mod models;

use std::error::Error;
use std::rc::Rc;
use std::time::Duration;

use generated::{AdditionalDrivesModel, AppWindow, ContentStatus};
use slint::ComponentHandle;
use tokio::sync::oneshot;

use app_state::AppState;
use atomic_borrow::AtomicBorrow;

async fn complicated_calculation(value: Rc<i32>) -> i32 {
    let (tx, rx) = oneshot::channel();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(1));
        let _ = tx.send(());
    });
    let _ = rx.await;
    *value + 1
}

async fn update_value(app_state: Rc<AtomicBorrow<AppState>>) {
    let value = app_state.with(|app_state| app_state.value.clone());
    let weak = Rc::downgrade(&value);
    let new_value = complicated_calculation(value).await;
    if let Some(value) = weak.upgrade() {
        app_state.with_mut(|app_state| {
            if Rc::ptr_eq(&value, &app_state.value) {
                app_state.value = Rc::new(new_value);
                app_state.window.set_additional_drives(AdditionalDrivesModel::new(
                    vec![],
                    new_value.to_string(),
                    ContentStatus::Error,
                ));
            }
        });
    }
}

fn set_callbacks(app_window: &AppWindow, app_state: Rc<AtomicBorrow<AppState>>) {
    let copy = app_state.clone();
    app_window.on_update_device_list(move || {
        let _ = slint::spawn_local(update_value(copy.clone()));
    });
}

fn main() -> Result<(), Box<dyn Error>> {
    let _ = slint::BackendSelector::new().backend_name("winit".into()).renderer_name("skia".into()).select();

    let app_window = AppWindow::new()?;
    let app_state = Rc::new(AtomicBorrow::new(AppState::new(app_window.clone_strong())));
    set_callbacks(&app_window, app_state.clone());
    app_window.run()?;

    Ok(())
}
