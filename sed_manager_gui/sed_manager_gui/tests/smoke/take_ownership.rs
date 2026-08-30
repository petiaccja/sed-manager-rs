use std::sync::Arc;
use std::time::{Duration, Instant};

use i_slint_backend_testing::ElementHandle;
use sed_async::{PolyRuntime, Runtime, SlintRuntime};
use sed_manager_gui::{app::App, toast::ToastQueue};
use sed_manager_gui_slint as ui;
use sed_telemetry::{create_otlp_provider, init_otlp_subscriber, init_stdout_subscriber};
use slint::platform::PointerEventButton;
use slint::{ComponentHandle as _, Model as _};

use crate::element_handle_ext::ElementHandleEx;

const TEST_PASSWORD: &str = "test-password-1234";

// `sed_async::sleep` needs to run inside a Tokio reactor, but this test's async block runs on
// the Slint event loop instead (via `slint::spawn_local`) - so the delay itself is spawned onto
// `runtime` and awaited from there, same as `Command`'s `.run()` methods do in `command.rs`.
async fn wait_until(runtime: Arc<PolyRuntime>, mut condition: impl FnMut() -> bool, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while !condition() {
        if Instant::now() >= deadline {
            return false;
        }
        runtime.sleep(Duration::from_millis(20)).await;
    }
    true
}

fn find_one(ui: &ui::MainWindow, id: &str) -> ElementHandle {
    ElementHandle::find_by_accessible_id(ui, id)
        .next()
        .unwrap_or_else(|| panic!("no element found with accessible-id {id:?}"))
}

#[test]
fn scan_select_take_ownership() {
    let _tracing_guard = match create_otlp_provider() {
        Ok(provider) => init_otlp_subscriber(provider),
        Err(_) => init_stdout_subscriber(),
    };

    i_slint_backend_testing::init_integration_test_with_system_time();

    slint::spawn_local(async move {
        let runtime = Arc::new(PolyRuntime::Slint(SlintRuntime));
        let ui = ui::MainWindow::new().unwrap();
        let notification_queue = ToastQueue::new(ui.clone_strong());
        let _app = App::new(ui.clone_strong(), notification_queue, runtime.clone());

        // Scan for devices; the virtual device is always included in debug builds.
        find_one(&ui, "scan-button").single_click(PointerEventButton::Left).await;
        assert!(
            wait_until(
                runtime.clone(),
                || ElementHandle::find_by_accessible_id(&ui, "device-side-bar-item-0").next().is_some(),
                Duration::from_secs(5)
            )
            .await,
            "virtual device did not appear in the sidebar after scanning"
        );

        // Select the virtual device.
        find_one(&ui, "device-side-bar-item-0").single_click(PointerEventButton::Left).await;
        assert!(
            wait_until(
                runtime.clone(),
                || ElementHandle::find_by_accessible_id(&ui, "activity-take-ownership").next().is_some(),
                Duration::from_secs(5)
            )
            .await,
            "\"Take ownership\" activity did not appear after selecting the device"
        );

        // Switch to the "Take ownership" activity and fill in the password form.
        find_one(&ui, "activity-take-ownership").single_click(PointerEventButton::Left).await;
        find_one(&ui, "new-password").set_accessible_value(TEST_PASSWORD);
        find_one(&ui, "repeat-password").set_accessible_value(TEST_PASSWORD);

        // Submit and wait for the success toast, confirming the full click -> sed_tper ->
        // VirtualDevice -> UI round trip actually worked.
        find_one(&ui, "take-ownership-submit").single_click(PointerEventButton::Left).await;
        let took_ownership = wait_until(
            runtime.clone(),
            || ui.global::<ui::ToastQueue>().get_queue().iter().any(|item| item.toast.title == "Taken ownership"),
            Duration::from_secs(10),
        )
        .await;
        assert!(took_ownership, "did not see a \"Taken ownership\" toast after submitting");

        slint::quit_event_loop().unwrap();
    })
    .unwrap();
    slint::run_event_loop().unwrap();
}
