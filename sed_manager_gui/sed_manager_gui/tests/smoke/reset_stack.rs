//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::sync::Arc;
use std::time::Duration;

use sed_async::{PolyRuntime, SlintRuntime};
use sed_manager::Host;
use sed_manager_gui::{app::App, toast::ToastQueue};
use sed_manager_gui_slint as ui;
use slint::ComponentHandle as _;
use slint::platform::PointerEventButton;

pub use crate::driver::Driver;
use crate::test_utils::run_test;

use PointerEventButton::Left;

/// This test exercises the following functionality:
/// - Reset stack
#[test]
fn reset_stack() {
    run_test(|| async {
        let runtime = Arc::new(PolyRuntime::Slint(SlintRuntime));
        let ui = ui::MainWindow::new().unwrap();
        let toast_queue = ToastQueue::new(ui.clone_strong());
        let host = Arc::new(Host::new(runtime.clone()));
        let _app = App::new(ui.clone_strong(), toast_queue, host, runtime.clone());

        let ui = Driver::new(ui, Duration::from_secs(5));

        // Scan for devices. The virtual device is sorted first because the
        // others are blocked by file OS access control.
        ui.single_click("scan-button", Left).await.unwrap();
        ui.check_appearance("device-side-bar-item-0").await.unwrap();

        // Select the virtual device.
        ui.single_click("device-side-bar-item-0", Left).await.unwrap();

        // Click "Reset stack" and wait for the success toast. The button is disabled
        // while the commands issued when the device was opened are still running. The
        // click is invoked synchronously, so the button can't get disabled mid-click.
        ui.check_enabled("activity-reset-stack").await.unwrap();
        ui.invoke_default_action("activity-reset-stack").unwrap();
        ui.check_appearance_toast("Stack has been reset").await.unwrap();
    });
}
