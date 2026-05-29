#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod display_ui;
mod app;
mod toast;

use sed_manager_gui_slint::{self as ui};
use slint::ComponentHandle as _;

use crate::app::App;

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ui = ui::MainWindow::new()?;
    let notification_queue = toast::ToastQueue::new(ui.clone_strong());
    let _main_app = App::new(ui.clone_strong(), notification_queue.clone());
    ui.invoke_scan();
    ui.run()?;
    Ok(())
}
