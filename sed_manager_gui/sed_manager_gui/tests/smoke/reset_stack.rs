//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::sync::Arc;

use sed_async::{PolyRuntime, SlintRuntime};
use sed_manager::Host;
use sed_manager_gui::{app::App, toast::ToastQueue};
use sed_manager_gui_slint as ui;
use slint::platform::PointerEventButton;
use slint::{ComponentHandle as _, Model as _};

use crate::element_handle_ext::ElementHandleEx;
use crate::test_utils::{assert_present, assert_toast, find_element, run_test};

/// This test exercises the following functionality:
/// - Reset stack
#[test]
fn reset_stack() {
    run_test(|| async {
        let runtime = Arc::new(PolyRuntime::Slint(SlintRuntime));
        let ui = ui::MainWindow::new().unwrap();
        let notification_queue = ToastQueue::new(ui.clone_strong());
        let host = Arc::new(Host::new(runtime.clone()));
        let _app = App::new(ui.clone_strong(), notification_queue, host, runtime.clone());

        // Scan for devices; the virtual device is always included in debug builds.
        find_element(&ui, "scan-button").single_click(PointerEventButton::Left).await;
        assert_present!(&ui, "device-side-bar-item-0");

        // Select the virtual device.
        find_element(&ui, "device-side-bar-item-0").single_click(PointerEventButton::Left).await;
        assert_present!(&ui, "activity-reset-stack");

        // Click "Reset stack" and wait for the success toast.
        find_element(&ui, "activity-reset-stack").single_click(PointerEventButton::Left).await;
        assert_toast!(&ui, "Stack has been reset");
    });
}
