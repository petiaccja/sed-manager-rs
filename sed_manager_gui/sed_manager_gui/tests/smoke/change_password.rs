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
use slint::ComponentHandle as _;
use slint::platform::PointerEventButton::Left;

use crate::driver::Driver;
use crate::test_utils::run_test;

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

        let ui = Driver::new(ui, Duration::from_secs(5));

        // Scan for devices; the virtual device is always included in debug builds.
        ui.single_click("scan-button", Left).await.unwrap();
        ui.check_appearance("device-side-bar-item-0").await.unwrap();

        // Select the virtual device.
        ui.single_click("device-side-bar-item-0", Left).await.unwrap();
        ui.check_appearance("activity-change-password").await.unwrap();

        // Switch to the "Change password" activity. Selecting it triggers a silent, background
        // listing of the admin SP's authorities; wait for that to populate the authority combo
        // box before interacting with it (there's no toast to wait on, since it's silent).
        ui.single_click("activity-change-password", Left).await.unwrap();
        ui.check_condition("change-password-authority", |element| {
            element.accessible_value().is_some_and(|value| !value.is_empty())
        })
        .await
        .unwrap();

        // Select the SID authority (the security provider combo box already defaults to the
        // admin SP, which is the one that owns the SID authority).
        ui.single_click("change-password-authority", Left).await.unwrap();
        ui.select_combo_box_item("change-password-authority", "SID").unwrap();

        // Fill in the current (factory MSID) password and the new password.
        let initial_sid_password = INITIAL_SID_PASSWORD;
        let initial_password = str::from_utf8(&initial_sid_password).unwrap();
        ui.enter_text("change-password-current-password", initial_password).await.unwrap();
        ui.enter_text("new-password", TEST_PASSWORD).await.unwrap();
        ui.enter_text("repeat-password", TEST_PASSWORD).await.unwrap();

        // Submit and wait for the success toast. The button is disabled while the TPer or the
        // session is busy, so it's invoked synchronously right after it's found enabled, as
        // opposed to an asynchronous mouse click, during which it may get disabled again.
        ui.check_enabled("change-password-submit").await.unwrap();
        ui.invoke_default_action("change-password-submit").unwrap();
        ui.check_appearance_toast("Password changed").await.unwrap();
    });
}
