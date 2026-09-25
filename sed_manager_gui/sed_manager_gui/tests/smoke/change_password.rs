//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_async::{PolyRuntime, SlintRuntime};
use sed_manager::Host;
use sed_manager_gui::{app::App, toast::ToastQueue};
use sed_manager_gui_slint as ui;
use sed_virtual_device::INITIAL_SID_PASSWORD;
use slint::platform::PointerEventButton;
use slint::{ComponentHandle as _, Model as _};
use std::sync::Arc;

use crate::element_handle_ext::ElementHandleEx;
use crate::test_utils::{assert_present, assert_toast, find_element, run_test, select_combo_box_item};

const TEST_PASSWORD: &str = "test-password-1234";

/// This test changes the SID authority's password on a freshly-factory device, without ever
/// taking ownership. It exploits the fact that the virtual device's SID password starts out equal
/// to the (hardcoded, well-known) MSID password.
///
/// This test exercises the following functionality:
/// - Change password
#[test]
fn change_password() {
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
        assert_present!(&ui, "activity-change-password");

        // Switch to the "Change password" activity. Selecting it triggers a silent, background
        // listing of the admin SP's authorities, but refreshing explicitly gives us a toast to
        // wait on, so we know the authority combo box is populated before we touch it.
        find_element(&ui, "activity-change-password").single_click(PointerEventButton::Left).await;
        find_element(&ui, "change-password-refresh").single_click(PointerEventButton::Left).await;
        assert_toast!(&ui, "Admin authorities updated");

        // Select the SID authority (the security provider combo box already defaults to the
        // admin SP, which is the one that owns the SID authority).
        find_element(&ui, "change-password-authority").single_click(PointerEventButton::Left).await;
        select_combo_box_item(&ui, "change-password-authority", "SID");

        // Fill in the current (factory MSID) password and the new password.
        let initial_sid_password = INITIAL_SID_PASSWORD;
        let initial_password = str::from_utf8(&initial_sid_password).unwrap();
        find_element(&ui, "change-password-current-password").set_accessible_value(initial_password);
        find_element(&ui, "new-password").set_accessible_value(TEST_PASSWORD);
        find_element(&ui, "repeat-password").set_accessible_value(TEST_PASSWORD);

        // Submit and wait for the success toast.
        find_element(&ui, "change-password-submit").single_click(PointerEventButton::Left).await;
        assert_toast!(&ui, "Password changed");
    });
}
