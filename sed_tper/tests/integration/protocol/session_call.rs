use std::sync::Arc;

use googletest::{assert_that, matchers::*};
use tokio::spawn;
use tokio::time::timeout;
use tracing::{Instrument, instrument};

use sed_device::Error as DeviceError;
use sed_device::mock_device::MockDevice;
use sed_packet::Bytes;
use sed_packet::session_id::SessionId;
use sed_packet::token::{Command, ToTokens};
use sed_spec::methods::{MethodCall, MethodResult, MethodStatus, Properties, Random, RandomResult};
use sed_spec::preconfig::core::shared::invoking_id::THIS_SP;
use sed_spec::preconfig::core::shared::method_id::RANDOM;
use sed_telemetry::{WithTracing, with_tracing};
use sed_tper::Error;
use sed_tper::protocol::Protocol;

use crate::utility::{
    TEST_TIMEOUT, method_call_event, method_call_fail_event, method_return_event, method_return_fail_event,
};

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

    controller.spawn(session_id, Properties::INITIAL);
    let result = timeout(TEST_TIMEOUT, controller.call(session_id, random_call.to_tokens().unwrap())).await;
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

    controller.spawn(session_id, Properties::INITIAL);
    let result = timeout(TEST_TIMEOUT, controller.call(session_id, random_call.to_tokens().unwrap())).await;
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

    controller.spawn(session_id, Properties::INITIAL);
    let result = timeout(TEST_TIMEOUT, controller.call(session_id, random_call.to_tokens().unwrap())).await;
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
async fn send_session_method_eos(_with_tracing: WithTracing) {
    let com_id = 1u16;
    let session_id = SessionId { hsn: 1, tsn: 3 };

    let scenario = [
        method_call_event(com_id, session_id.hsn, session_id.tsn, &Command::EndOfSession),
        method_return_event(com_id, session_id.hsn, session_id.tsn, &Command::EndOfSession),
    ];

    let device = Arc::new(MockDevice::new(scenario.into_iter()));
    let (protocol, controller) = Protocol::new(com_id, 0, device.clone());
    let handle = spawn(protocol.run().in_current_span());

    controller.spawn(session_id, Properties::INITIAL);
    let result = timeout(TEST_TIMEOUT, controller.call(session_id, Command::EndOfSession.to_tokens().unwrap())).await;

    drop(controller);
    let handle_result = handle.await;

    assert_that!(result, ok(ok(ok(eq(&Command::EndOfSession.to_tokens().unwrap())))));
    assert_that!(handle_result, ok(anything()));
    device.check();
}
