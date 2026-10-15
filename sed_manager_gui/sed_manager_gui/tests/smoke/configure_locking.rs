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
use crate::test_utils::{
    assert_absent, assert_enabled, assert_present, assert_toast, enter_text, find_element, get_text, prevent_clipping,
    run_test, select_combo_box_item, sleep_until_condition,
};

const TEST_PASSWORD: &str = "test-password-1234";

/// This test takes ownership, activates locking, logs into the locking SP as "Admin1" to reach
/// the "Configure locking" page, changes and saves a locking range and an authority, then logs
/// back out.
///
/// This test exercises the following functionality:
/// - Take ownership
/// - Activate locking
/// - Login (to the locking SP)
/// - Listing and writing locking ranges
/// - Listing and writing authorities
/// - Logout
#[test]
fn enter_config_page() {
    run_test(|| async {
        let runtime = Arc::new(PolyRuntime::Slint(SlintRuntime));
        let ui = ui::MainWindow::new().unwrap();
        prevent_clipping(&ui);
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
        assert_enabled!(&ui, "login-submit");
        find_element(&ui, "login-submit").single_click(PointerEventButton::Left).await;

        // A successful login switches the "Configure locking" page from the login panel to the
        // locking SP's tabs, landing on "Locking ranges" first, which has its own "Logout" button.
        // Wait for that before proceeding, so the login is confirmed to actually have gone through.
        assert_present!(&ui, "logout-submit");

        // Toggle "Read locking enabled" of the first non-global range (index 0
        // is the Global Range) and save the ranges.
        let checkbox_id = "locking-range-read-lock-enabled-1";
        assert_present!(&ui, checkbox_id);
        assert_eq!(find_element(&ui, checkbox_id).accessible_checked(), Some(false));
        find_element(&ui, checkbox_id).invoke_accessible_default_action();

        // Also move the range to an unaligned span. The UI should round the
        // span down to the alignment granularity (8 on the virtual device),
        // otherwise the device rejects the update.
        // The sectors are 512 bytes, so 1000 and 2000 LBAs are displayed as
        // 500 and 1000 KiB, whereas the unaligned values would be displayed as
        // 500.5 and 1001.5 KiB.
        enter_text(&ui, "locking-range-start-1", "1001").await;
        enter_text(&ui, "locking-range-end-1", "2003").await;
        assert_eq!(get_text(&ui, "locking-range-start-1").as_deref(), Some("500 KiB"));
        assert_eq!(get_text(&ui, "locking-range-end-1").as_deref(), Some("1000 KiB"));

        // Background commands started by the login may briefly make the session
        // busy, which disables the button. The click is therefore invoked
        // synchronously right after it's found enabled, as opposed to an
        // asynchronous mouse click, during which it may get disabled again.
        assert_enabled!(&ui, "save-ranges-submit");
        find_element(&ui, "save-ranges-submit").invoke_accessible_default_action();
        assert_toast!(&ui, "Locking ranges written to device");

        // The ranges are reloaded from the device after saving, so once there
        // are no unsaved changes, the UI reflects the state of the device.
        sleep_until_condition(
            || find_element(&ui, "save-ranges-submit").accessible_enabled() == Some(false),
            Duration::from_secs(5),
        )
        .await
        .unwrap_or_else(|_| panic!("the locking ranges were not reloaded"));

        // Reloading replaces the model, and the rows are recreated lazily.
        assert_present!(&ui, checkbox_id);
        assert_eq!(find_element(&ui, checkbox_id).accessible_checked(), Some(true));
        assert_eq!(get_text(&ui, "locking-range-start-1").as_deref(), Some("500 KiB"));
        assert_eq!(get_text(&ui, "locking-range-end-1").as_deref(), Some("1000 KiB"));

        // Switch to the "Authorities" tab and enable "User1", which is
        // disabled by default on a freshly-activated locking SP.
        ElementHandle::find_by_accessible_label(&ui, "Authorities")
            .next()
            .expect("the authorities tab should be present")
            .single_click(PointerEventButton::Left)
            .await;
        let checkbox_id = "authority-enabled-User1";
        assert_present!(&ui, checkbox_id);
        assert_eq!(find_element(&ui, checkbox_id).accessible_checked(), Some(false));
        find_element(&ui, checkbox_id).invoke_accessible_default_action();

        // Same as for the ranges: the click is invoked synchronously, so the
        // button can't get disabled by background commands mid-click.
        assert_enabled!(&ui, "save-authorities-submit");
        find_element(&ui, "save-authorities-submit").invoke_accessible_default_action();
        assert_toast!(&ui, "Authorities written to device");

        // The authorities are reloaded from the device after saving. The
        // reload must not end the session, so the user must stay logged in.
        assert_enabled!(&ui, "save-authorities-submit");
        assert_present!(&ui, "logout-submit");
        assert_present!(&ui, checkbox_id);
        assert_eq!(find_element(&ui, checkbox_id).accessible_checked(), Some(true));

        // Log back out and wait for the login panel to reappear.
        find_element(&ui, "logout-submit").invoke_accessible_default_action();
        assert_present!(&ui, "login-submit");

        // Only enabled authorities are listed in the login panel, so "User1"
        // is now available to log in with.
        find_element(&ui, "login-authority").single_click(PointerEventButton::Left).await;
        select_combo_box_item(&ui, "login-authority", "User1");
        assert_eq!(find_element(&ui, "login-authority").accessible_value().as_deref(), Some("User1"));
    });
}
