use std::sync::Arc;

use googletest::{assert_that, matchers::*};
use tokio::spawn;
use tokio::time::timeout;
use tracing::{Instrument, instrument};

use sed_device::Error as DeviceError;
use sed_device::mock_device::MockDevice;
use sed_packet::session_id::SessionId;
use sed_packet::token::ToTokens;
use sed_spec::methods::{MethodCall, MethodStatus, StartSession, SyncSession};
use sed_spec::preconfig::core::shared::invoking_id::SESSION_MANAGER;
use sed_spec::preconfig::core::shared::sm_method_id::{START_SESSION, SYNC_SESSION};
use sed_spec::preconfig::opal_2::admin::sp;
use sed_telemetry::{WithTracing, with_tracing};
use sed_tper::error::Error;
use sed_tper::protocol::Protocol;

use crate::utility::{
    TEST_TIMEOUT, method_call_event, method_call_fail_event, method_return_event, method_return_fail_event,
};

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn send_management_method_success(_with_tracing: WithTracing) {
    let com_id = 1u16;

    let start_session = MethodCall {
        invoking_id: SESSION_MANAGER,
        method_id: START_SESSION,
        parameters: StartSession::new(1, sp::ADMIN),
        status: MethodStatus::Success,
    };

    let sync_session = MethodCall {
        invoking_id: SESSION_MANAGER,
        method_id: SYNC_SESSION,
        parameters: SyncSession::new(1, 0),
        status: MethodStatus::SPBusy,
    };

    let scenario = [
        method_call_event(com_id, 0, 0, &start_session),
        method_return_event(com_id, 0, 0, &sync_session),
    ];

    let device = Arc::new(MockDevice::new(scenario.into_iter()));
    let (protocol, controller) = Protocol::new(com_id, 0, device.clone());
    let handle = spawn(protocol.run().in_current_span());
    let result =
        timeout(TEST_TIMEOUT, controller.call(SessionId::MANAGEMENT, start_session.to_tokens().unwrap())).await;
    drop(controller);
    let handle_result = handle.await;

    assert_that!(result, ok(ok(ok(eq(&sync_session.to_tokens().unwrap())))));
    assert_that!(handle_result, ok(anything()));
    device.check();
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn send_management_method_iface_send_fail(_with_tracing: WithTracing) {
    let com_id = 1u16;

    let start_session = MethodCall {
        invoking_id: SESSION_MANAGER,
        method_id: START_SESSION,
        parameters: StartSession::new(1, sp::ADMIN),
        status: MethodStatus::Success,
    };

    let scenario = [method_call_fail_event(
        com_id,
        0,
        0,
        &start_session,
        DeviceError::NotSupported,
    )];

    let device = Arc::new(MockDevice::new(scenario.into_iter()));
    let (protocol, controller) = Protocol::new(com_id, 0, device.clone());
    let handle = spawn(protocol.run().in_current_span());
    let result =
        timeout(TEST_TIMEOUT, controller.call(SessionId::MANAGEMENT, start_session.to_tokens().unwrap())).await;
    drop(controller);
    let handle_result = handle.await;

    assert_that!(result, ok(ok(err(eq(&Error::SecurityCommandFailed(DeviceError::NotSupported))))));
    assert_that!(handle_result, ok(anything()));
    device.check();
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn send_management_method_iface_recv_failed(_with_tracing: WithTracing) {
    let com_id = 1u16;

    let start_session = MethodCall {
        invoking_id: SESSION_MANAGER,
        method_id: START_SESSION,
        parameters: StartSession::new(1, sp::ADMIN),
        status: MethodStatus::Success,
    };

    let scenario = [
        method_call_event(com_id, 0, 0, &start_session),
        method_return_fail_event(com_id, DeviceError::NotSupported),
    ];

    let device = Arc::new(MockDevice::new(scenario.into_iter()));
    let (protocol, controller) = Protocol::new(com_id, 0, device.clone());
    let handle = spawn(protocol.run().in_current_span());
    let result =
        timeout(TEST_TIMEOUT, controller.call(SessionId::MANAGEMENT, start_session.to_tokens().unwrap())).await;
    drop(controller);
    let handle_result = handle.await;

    assert_that!(result, ok(ok(err(eq(&Error::TimedOut)))));
    assert_that!(handle_result, ok(anything()));
    device.check();
}
