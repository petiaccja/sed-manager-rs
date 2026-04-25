use std::sync::Arc;
use std::time::Duration;

use googletest::{assert_that, matchers::*};
use sed_packet::com_id::{HandleComIdRequest, HandleComIdResponse, HandleComIdResponseParams, StackResetStatus};
use tokio::spawn;
use tokio::time::timeout;
use tracing::{Instrument, instrument};

use sed_device::Error as DeviceError;
use sed_device::mock_device::MockDevice;
use sed_packet::Bytes;
use sed_packet::session_id::SessionId;
use sed_packet::token::ToTokens;
use sed_spec::methods::{
    MethodCall, MethodResult, MethodStatus, Properties, Random, RandomResult, StartSession, SyncSession,
};
use sed_spec::preconfig::core::shared::invoking_id::{SESSION_MANAGER, THIS_SP};
use sed_spec::preconfig::core::shared::method_id::RANDOM;
use sed_spec::preconfig::core::shared::sm_method_id::{START_SESSION, SYNC_SESSION};
use sed_spec::preconfig::opal_2::admin::sp;
use sed_telemetry::{WithTracing, with_tracing};
use sed_tper::error::Error;
use sed_tper::protocol::Protocol;

use crate::utility::{
    com_id_request_event, com_id_request_fail_event, com_id_response_event, com_id_response_fail_event,
    method_call_event, method_call_fail_event, method_return_event, method_return_fail_event,
};

const TIMEOUT: Duration = Duration::from_secs(5);
const SHORT_PROPERTIES: Properties = Properties {
    trans_timeout: Duration::from_millis(500),
    def_trans_timeout: Duration::from_millis(500),
    ..Properties::ASSUMED
};

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn shutdown(_with_tracing: WithTracing) {
    let device = Arc::new(MockDevice::new([].into_iter()));
    let (protocol, _) = Protocol::new(1, 0, device);
    let handle = spawn(protocol.run().in_current_span());
    assert_that!(tokio::time::timeout(Duration::from_secs(5), handle).await, ok(anything()));
}

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
    let result = timeout(TIMEOUT, controller.call(SessionId::MANAGEMENT, start_session.to_tokens().unwrap())).await;
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
    let result = timeout(TIMEOUT, controller.call(SessionId::MANAGEMENT, start_session.to_tokens().unwrap())).await;
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
    let result = timeout(TIMEOUT, controller.call(SessionId::MANAGEMENT, start_session.to_tokens().unwrap())).await;
    drop(controller);
    let handle_result = handle.await;

    assert_that!(result, ok(ok(err(eq(&Error::TimedOut)))));
    assert_that!(handle_result, ok(anything()));
    device.check();
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn send_session_method_success(_with_tracing: WithTracing) {
    let com_id = 1u16;
    let session_id = SessionId { hsn: 1, tsn: 3 };

    let random_call = MethodCall {
        invoking_id: THIS_SP,
        method_id: RANDOM.into(),
        parameters: Random { count: 4, buffer_out: None },
        status: MethodStatus::Success,
    };

    let random_result = MethodResult(Ok(RandomResult { result: Bytes(vec![0xCC; 4]) }));

    let scenario = [
        method_call_event(com_id, session_id.hsn, session_id.tsn, &random_call),
        method_return_event(com_id, session_id.hsn, session_id.tsn, &random_result),
    ];

    let device = Arc::new(MockDevice::new(scenario.into_iter()));
    let (protocol, controller) = Protocol::new(com_id, 0, device.clone());
    let handle = spawn(protocol.run().in_current_span());

    controller.spawn(session_id, SHORT_PROPERTIES);
    let result = timeout(TIMEOUT, controller.call(session_id, random_call.to_tokens().unwrap())).await;
    controller.delete(session_id);

    drop(controller);
    let handle_result = handle.await;

    assert_that!(result, ok(ok(ok(eq(&random_result.to_tokens().unwrap())))));
    assert_that!(handle_result, ok(anything()));
    device.check();
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn send_session_method_iface_send_failed(_with_tracing: WithTracing) {
    let com_id = 1u16;
    let session_id = SessionId { hsn: 1, tsn: 3 };

    let random_call = MethodCall {
        invoking_id: THIS_SP,
        method_id: RANDOM.into(),
        parameters: Random { count: 4, buffer_out: None },
        status: MethodStatus::Success,
    };

    let scenario = [method_call_fail_event(
        com_id,
        session_id.hsn,
        session_id.tsn,
        &random_call,
        DeviceError::NotSupported,
    )];

    let device = Arc::new(MockDevice::new(scenario.into_iter()));
    let (protocol, controller) = Protocol::new(com_id, 0, device.clone());
    let handle = spawn(protocol.run().in_current_span());

    controller.spawn(session_id, SHORT_PROPERTIES);
    let result = timeout(TIMEOUT, controller.call(session_id, random_call.to_tokens().unwrap())).await;
    controller.delete(session_id);

    drop(controller);
    let handle_result = handle.await;

    assert_that!(result, ok(ok(err(eq(&Error::SecurityCommandFailed(DeviceError::NotSupported))))));
    assert_that!(handle_result, ok(anything()));
    device.check();
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn send_session_method_iface_recv_failed(_with_tracing: WithTracing) {
    let com_id = 1u16;
    let session_id = SessionId { hsn: 1, tsn: 3 };

    let random_call = MethodCall {
        invoking_id: THIS_SP,
        method_id: RANDOM.into(),
        parameters: Random { count: 4, buffer_out: None },
        status: MethodStatus::Success,
    };

    let scenario = [
        method_call_event(com_id, session_id.hsn, session_id.tsn, &random_call),
        method_return_fail_event(com_id, DeviceError::NotSupported),
    ];

    let device = Arc::new(MockDevice::new(scenario.into_iter()));
    let (protocol, controller) = Protocol::new(com_id, 0, device.clone());
    let handle = spawn(protocol.run().in_current_span());

    controller.spawn(session_id, SHORT_PROPERTIES);
    let result = timeout(TIMEOUT, controller.call(session_id, random_call.to_tokens().unwrap())).await;
    controller.delete(session_id);

    drop(controller);
    let handle_result = handle.await;

    assert_that!(result, ok(ok(err(eq(&Error::TimedOut)))));
    assert_that!(handle_result, ok(anything()));
    device.check();
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn send_com_id_request_success(_with_tracing: WithTracing) {
    let com_id = 1u16;

    let request = HandleComIdRequest::stack_reset(0x0005, 0x0000);

    let response = HandleComIdResponse {
        com_id: 0x0005,
        com_id_ext: 0x0000,
        params: HandleComIdResponseParams::StackReset { available_data_length: 0, status: StackResetStatus::Success },
    };

    let scenario = [
        com_id_request_event(com_id, &request),
        com_id_response_event(com_id, &response),
    ];

    let device = Arc::new(MockDevice::new(scenario.into_iter()));
    let (protocol, controller) = Protocol::new(com_id, 0, device.clone());
    let handle = spawn(protocol.run().in_current_span());
    let result = timeout(TIMEOUT, controller.com_id_request(request)).await;
    drop(controller);
    let handle_result = handle.await;

    assert_that!(result, ok(ok(ok(eq(&response)))));
    assert_that!(handle_result, ok(anything()));
    device.check();
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn send_com_id_request_iface_send_failed(_with_tracing: WithTracing) {
    let com_id = 1u16;

    let request = HandleComIdRequest::stack_reset(0x0005, 0x0000);

    let scenario = [com_id_request_fail_event(
        com_id,
        &request,
        DeviceError::NotSupported,
    )];

    let device = Arc::new(MockDevice::new(scenario.into_iter()));
    let (protocol, controller) = Protocol::new(com_id, 0, device.clone());
    let handle = spawn(protocol.run().in_current_span());
    let result = timeout(TIMEOUT, controller.com_id_request(request)).await;
    drop(controller);
    let handle_result = handle.await;

    assert_that!(result, ok(ok(err(eq(&Error::SecurityCommandFailed(DeviceError::NotSupported))))));
    assert_that!(handle_result, ok(anything()));
    device.check();
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn send_com_id_request_iface_recv_failed(_with_tracing: WithTracing) {
    let com_id = 1u16;

    let request = HandleComIdRequest::stack_reset(0x0005, 0x0000);

    let scenario = [
        com_id_request_event(com_id, &request),
        com_id_response_fail_event(com_id, DeviceError::NotSupported),
    ];

    let device = Arc::new(MockDevice::new(scenario.into_iter()));
    let (protocol, controller) = Protocol::new(com_id, 0, device.clone());
    let handle = spawn(protocol.run().in_current_span());
    let result = timeout(TIMEOUT, controller.com_id_request(request)).await;
    drop(controller);
    let handle_result = handle.await;

    assert_that!(result, ok(ok(err(eq(&Error::TimedOut)))));
    assert_that!(handle_result, ok(anything()));
    device.check();
}
