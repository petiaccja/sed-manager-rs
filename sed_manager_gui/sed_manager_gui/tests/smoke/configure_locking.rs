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
use slint::platform::PointerEventButton;
use slint::{ComponentHandle as _, Model as _};

use crate::element_handle_ext::ElementHandleEx;
use crate::test_utils::{assert_absent, assert_present, assert_toast, find_element, run_test, sleep_until_condition};

const TEST_PASSWORD: &str = "test-password-1234";

/// This test takes ownership, activates locking, logs into the locking SP as "Admin1" to reach
/// the "Configure locking" page, then logs back out.
///
/// This test exercises the following functionality:
/// - Take ownership
/// - Activate locking
/// - Login (to the locking SP)
/// - Logout
#[test]
fn enter_config_page() {
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

        // Switch to the "Configure locking" activity. Selecting it triggers a silent, background
        // listing of the locking SP's authorities; wait for that to populate the login panel's
        // authority combo box before interacting with it (there's no toast to wait on here, since
        // the listing is silent).
        assert_present!(&ui, "activity-configure-locking");
        find_element(&ui, "activity-configure-locking").single_click(PointerEventButton::Left).await;
        sleep_until_condition(
            || find_element(&ui, "login-authority").accessible_value().is_some_and(|value| !value.is_empty()),
            Duration::from_secs(5),
        )
        .await
        .unwrap_or_else(|_| panic!("the locking SP's authorities did not load"));

        // Log in as "Admin1". The authority combo box already defaults to it, since it's the only
        // individual authority enabled by default on a freshly-activated locking SP. Activating
        // locking copies the current SID password into Admin1's credential, so it's the same
        // password used to take ownership, not the virtual device's raw factory default.
        find_element(&ui, "login-password").set_accessible_value(TEST_PASSWORD);
        find_element(&ui, "login-submit").single_click(PointerEventButton::Left).await;

        // A successful login switches the "Configure locking" page from the login panel to the
        // locking SP's tabs, landing on "Locking ranges" first, which has its own "Logout" button.
        // Wait for that before proceeding, so the login is confirmed to actually have gone through.
        assert_present!(&ui, "logout-submit");

        // Log back out and wait for the login panel to reappear.
        find_element(&ui, "logout-submit").single_click(PointerEventButton::Left).await;
        assert_present!(&ui, "login-submit");
    });
}
