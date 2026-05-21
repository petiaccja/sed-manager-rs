#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use sed_manager_gui_slint as ui;
use slint::ComponentHandle as _;

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ui = ui::MainWindow::new()?;

    ui.run()?;

    Ok(())
}
