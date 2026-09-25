//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::sync::Arc;
use std::time::Duration;

use i_slint_backend_testing::ElementHandle;
use sed_async::{PolyRuntime, SlintRuntime};
use sed_manager::Host;
use sed_manager_gui::{app::App, toast::ToastQueue};
use sed_manager_gui_slint as ui;
use slint::platform::PointerEventButton;
use slint::{ComponentHandle as _, Model as _};

use crate::element_handle_ext::ElementHandleEx;
use crate::ui_testing::{assert_present, assert_toast, find_element, run_test};

const TEST_PASSWORD: &str = "test-password-1234";

#[test]
fn scan_select_take_ownership() {
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
        assert_present!(&ui, "activity-take-ownership");

        // Switch to the "Take ownership" activity and fill in the password form.
        find_element(&ui, "activity-take-ownership").single_click(PointerEventButton::Left).await;
        find_element(&ui, "new-password").set_accessible_value(TEST_PASSWORD);
        find_element(&ui, "repeat-password").set_accessible_value(TEST_PASSWORD);

        // Submit and wait for the success toast.
        find_element(&ui, "take-ownership-submit").single_click(PointerEventButton::Left).await;
        assert_toast!(&ui, "Taken ownership");
    });
}
