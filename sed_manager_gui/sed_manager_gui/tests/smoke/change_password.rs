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
use sed_virtual_device::INITIAL_SID_PASSWORD;
use slint::platform::PointerEventButton;
use slint::{ComponentHandle as _, Model as _};

use crate::element_handle_ext::ElementHandleEx;
use crate::test_utils::{
    assert_present, assert_toast, find_element, run_test, select_combo_box_item, sleep_until_condition,
};

const TEST_PASSWORD: &str = "test-password-1234";

/// This is a narrow test just for the *change password* functionality. No need
/// to take ownership, we know the initial SID password for the virtual device.
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
        // listing of the admin SP's authorities; wait for that to populate the authority combo
        // box before interacting with it (there's no toast to wait on, since it's silent).
        find_element(&ui, "activity-change-password").single_click(PointerEventButton::Left).await;
        sleep_until_condition(
            || {
                find_element(&ui, "change-password-authority")
                    .accessible_value()
                    .is_some_and(|value| !value.is_empty())
            },
            Duration::from_secs(5),
        )
        .await
        .unwrap_or_else(|_| panic!("the admin SP's authorities did not load"));

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
