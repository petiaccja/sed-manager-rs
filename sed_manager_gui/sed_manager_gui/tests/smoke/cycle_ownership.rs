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
use crate::test_utils::{assert_absent, assert_present, assert_toast, find_element, run_test, select_combo_box_item};

const TEST_PASSWORD: &str = "test-password-1234";

/// This test cycles the ownership of the device, starting from factory ->
/// owned -> activated -> owned -> factory.
///
/// This test exercises the following functionalities:
/// - Take ownership
/// - Activate locking
/// - Revert (locking & everything)
#[test]
fn cycle_ownership() {
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

        // A successful action triggers a background re-discovery, which navigates the sidebar
        // back to the "Discovery" activity once it completes. Wait for that before switching
        // activities again, or the switch may be undone by the pending re-discovery.
        assert_absent!(&ui, "new-password");

        // Switch to the "Activate locking" activity and submit the owner password.
        find_element(&ui, "activity-activate-locking").single_click(PointerEventButton::Left).await;
        find_element(&ui, "activate-locking-password").set_accessible_value(TEST_PASSWORD);
        find_element(&ui, "activate-locking-submit").single_click(PointerEventButton::Left).await;
        assert_toast!(&ui, "Locking activated");
        assert_absent!(&ui, "activate-locking-password");

        // Switch to the "Revert device" activity and revert just the locking SP.
        find_element(&ui, "activity-revert-device").single_click(PointerEventButton::Left).await;
        find_element(&ui, "revert-password").set_accessible_value(TEST_PASSWORD);
        find_element(&ui, "revert-submit").single_click(PointerEventButton::Left).await;
        assert_toast!(&ui, "Reverted device successfully");
        assert_absent!(&ui, "revert-password");
        // Reverting the locking SP disables locking again, so "Activate locking" reappears.
        assert_present!(&ui, "activity-activate-locking");

        // Switch to the "Revert device" activity again, select "Everything" as the scope, and
        // revert the whole device back to its factory state.
        find_element(&ui, "activity-revert-device").single_click(PointerEventButton::Left).await;
        find_element(&ui, "revert-scope").single_click(PointerEventButton::Left).await;
        select_combo_box_item(&ui, "revert-scope", "Everything");
        find_element(&ui, "revert-password").set_accessible_value(TEST_PASSWORD);
        find_element(&ui, "revert-submit").single_click(PointerEventButton::Left).await;
        assert_toast!(&ui, "Reverted device successfully");
        // Reverting the whole device restores the factory MSID password, so "Take ownership" reappears.
        assert_present!(&ui, "activity-take-ownership");
    });
}
