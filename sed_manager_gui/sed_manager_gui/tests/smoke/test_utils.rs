//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

//! Utilities for UI testing.

use std::panic::{AssertUnwindSafe, resume_unwind};
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use futures::FutureExt;
use sed_async::{Runtime, SlintRuntime};

static EVENT_LOOP: OnceLock<std::thread::JoinHandle<()>> = OnceLock::new();
static TEST_LOCK: Mutex<()> = Mutex::new(());

pub struct TimeoutError;

/// Run a GUI test and panic if the test fails.
pub fn run_test<T, F>(test: T)
where
    T: FnOnce() -> F + Send + 'static,
    F: Future + 'static,
    F::Output: Send,
{
    // Let only one test run at a time. The slint backend seems to get
    // overwhelmed with parallel tests and the tests start timing out.
    // A failed test poisons the lock, but that doesn't affect other tests.
    let _test_guard = TEST_LOCK.lock().unwrap_or_else(PoisonError::into_inner);

    // Shouldn't use `macro_support` items normally, but the crates are in the
    // same workspace so it's not that bad.
    let _otlp_flush_guard = sed_telemetry::macro_support::with_tracing();
    init_event_loop();

    let (result_tx, result_rx) = oneshot::channel();
    let (handle_tx, handle_rx) = oneshot::channel();
    slint::invoke_from_event_loop(move || {
        let handle = slint::spawn_local(async move {
            let result = AssertUnwindSafe(test()).catch_unwind().await;
            result_tx.send(result).unwrap();
        })
        .unwrap();
        handle_tx.send(handle).unwrap();
    })
    .unwrap();

    let handle = handle_rx.recv_timeout(Duration::from_secs(15)).unwrap();
    let result = result_rx.recv_timeout(Duration::from_secs(120));

    match result {
        Ok(Ok(_)) => (),
        Ok(Err(panic)) => resume_unwind(panic),
        Err(_) => {
            handle.abort();
            panic!("the test timed out");
        }
    }
}

/// Initializes the event loop once per process using the [`EVENT_LOOP`] static.
fn init_event_loop() {
    EVENT_LOOP.get_or_init(|| {
        let handle = std::thread::spawn(|| {
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
