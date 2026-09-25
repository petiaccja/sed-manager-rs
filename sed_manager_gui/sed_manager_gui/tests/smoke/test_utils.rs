//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

//! Utilities for UI testing.

use std::panic::{AssertUnwindSafe, resume_unwind};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use futures::FutureExt;
use i_slint_backend_testing::{ElementHandle, ElementRoot};
use sed_async::{Runtime, SlintRuntime};
use sed_telemetry::{create_otlp_provider, init_otlp_subscriber, init_stdout_subscriber};
use slint::ComponentHandle;
use slint::platform::PointerEventButton;

use crate::element_handle_ext::ElementHandleEx;

static EVENT_LOOP: OnceLock<std::thread::JoinHandle<()>> = OnceLock::new();

pub struct TimeoutError;

/// Run a GUI test and panic if the test fails.
pub fn run_test<T, F>(test: T)
where
    T: FnOnce() -> F + Send + 'static,
    F: Future + 'static,
    F::Output: Send,
{
    init_event_loop();

    let (tx, rx) = oneshot::channel();
    slint::invoke_from_event_loop(move || {
        slint::spawn_local(async move {
            let result = AssertUnwindSafe(test()).catch_unwind().await;
            tx.send(result).unwrap();
        })
        .unwrap();
    })
    .unwrap();

    let result = rx.recv().unwrap();

    match result {
        Ok(_) => (),
        Err(panic) => resume_unwind(panic),
    }
}

/// Initializes the event loop once per process using the [`EVENT_LOOP`] static.
fn init_event_loop() {
    EVENT_LOOP.get_or_init(|| {
        let handle = std::thread::spawn(|| {
            let _tracing_guard = match create_otlp_provider() {
                Ok(provider) => init_otlp_subscriber(provider),
                Err(_) => init_stdout_subscriber(),
            };

            i_slint_backend_testing::init_integration_test_with_system_time();
            slint::run_event_loop_until_quit().unwrap();
        });

        // Wait until the spawned thread eventually starts running the event loop.
        // Unfortunately the only way to poll this is by repeatedly trying to spawn
        // something on the event loop.
        while slint::invoke_from_event_loop(|| {}).is_err() {
            std::thread::yield_now();
        }

        handle
    });
}

/// Sleep until the provided condition becomes true or the timeout expires.
pub async fn sleep_until_condition(mut condition: impl FnMut() -> bool, timeout: Duration) -> Result<(), TimeoutError> {
    let deadline = Instant::now() + timeout;
    while !condition() {
        if Instant::now() >= deadline {
            return Err(TimeoutError);
        }
        SlintRuntime.sleep(Duration::from_millis(20)).await;
    }
    Ok(())
}

/// Find the first element by its ID. If no element is found, panic.
pub fn find_element(ui: &impl ElementRoot, id: &str) -> ElementHandle {
    ElementHandle::find_by_accessible_id(ui, id)
        .next()
        .unwrap_or_else(|| panic!("no element found with accessible-id \"{id}\""))
}

/// Selects an item from a combo box's popup, which must already be open.
///
/// `i-slint-backend-testing` reports the position of elements inside a popup relative to the
/// popup's own origin rather than to the window, so a plain `ElementHandle::single_click()` on
/// the popup item misses it. Composing the combo box's own (correctly reported) absolute position
/// with the item's (incorrectly relative) position happens to cancel out the error and land back
/// on the right spot.
pub fn select_combo_box_item(ui: &impl ComponentHandle, combo_box_id: &str, item_label: &str) {
    let combo_box_pos = find_element(ui, combo_box_id).absolute_position();
    let item = ElementHandle::find_by_accessible_label(ui, item_label).next().unwrap();
    let item_pos = item.absolute_position();
    let item_size = item.size();
    let click_pos = slint::LogicalPosition::new(
        combo_box_pos.x + item_pos.x + item_size.width / 2.0,
        combo_box_pos.y + item_pos.y + item_size.height / 2.0,
    );

    let window = ui.window();
    window.dispatch_event(slint::platform::WindowEvent::PointerMoved { position: click_pos });
    window.dispatch_event(slint::platform::WindowEvent::PointerPressed {
        position: click_pos,
        button: PointerEventButton::Left,
    });
    window.dispatch_event(slint::platform::WindowEvent::PointerReleased {
        position: click_pos,
        button: PointerEventButton::Left,
    });
}

/// Assert that a UI element is present or appears within a short time.
macro_rules! assert_present {
    ($ui:expr, $id:expr) => {
        let result = $crate::test_utils::sleep_until_condition(
            || ElementHandle::find_by_accessible_id($ui, $id.as_ref()).next().is_some(),
            Duration::from_secs(5),
        )
        .await;
        match result {
            Ok(_) => (),
            Err(_) => panic!("the ui element \"{}\" did not appear", $id),
        }
    };
}

/// Assert that a UI element is absent or disappears within a short time.
macro_rules! assert_absent {
    ($ui:expr, $id:expr) => {
        let result = $crate::test_utils::sleep_until_condition(
            || ElementHandle::find_by_accessible_id($ui, $id.as_ref()).next().is_none(),
            Duration::from_secs(5),
        )
        .await;
        match result {
            Ok(_) => (),
            Err(_) => panic!("the ui element \"{}\" did not disappear", $id),
        }
    };
}

/// Assert that a toast message is present or appears within a short time.
macro_rules! assert_toast {
    ($ui:expr, $title:expr) => {
        let result = $crate::test_utils::sleep_until_condition(
            || $ui.global::<ui::ToastQueue>().get_queue().iter().any(|item| item.toast.title == $title),
            Duration::from_secs(5),
        )
        .await;
        match result {
            Ok(_) => (),
            Err(_) => panic!("the toast message with title \"{}\" did not appear", $title),
        }
    };
}

pub(crate) use assert_absent;
pub(crate) use assert_present;
pub(crate) use assert_toast;
