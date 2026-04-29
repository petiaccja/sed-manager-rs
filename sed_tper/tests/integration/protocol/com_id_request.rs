use std::sync::Arc;

use googletest::{assert_that, matchers::*};
use sed_packet::com_id::{ComIdRequest, ComIdResponse, ComIdResponsePayload, StackResetStatus};
use tokio::spawn;
use tokio::time::timeout;
use tracing::{Instrument, instrument};

use sed_device::Error as DeviceError;
use sed_device::mock_device::MockDevice;
use sed_telemetry::{WithTracing, with_tracing};
use sed_tper::error::Error;
use sed_tper::protocol::Protocol;

use crate::utility::{
    TEST_TIMEOUT, com_id_request_event, com_id_request_fail_event, com_id_response_event, com_id_response_fail_event,
};

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn send_com_id_request_success(_with_tracing: WithTracing) {
    let com_id = 1u16;

    let request = ComIdRequest::stack_reset(0x0005, 0x0000);

    let response = ComIdResponse {
        com_id: 0x0005,
        com_id_ext: 0x0000,
        payload: ComIdResponsePayload::StackReset { available_data_length: 0, status: StackResetStatus::Success },
    };

    let scenario = [
        com_id_request_event(com_id, &request),
        com_id_response_event(com_id, &response),
    ];

    let device = Arc::new(MockDevice::new(scenario.into_iter()));
    let (protocol, controller) = Protocol::new(com_id, 0, device.clone());
    let handle = spawn(protocol.run().in_current_span());
    let result = timeout(TEST_TIMEOUT, controller.com_id_request(request)).await;
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

    let request = ComIdRequest::stack_reset(0x0005, 0x0000);

    let scenario = [com_id_request_fail_event(
        com_id,
        &request,
        DeviceError::NotSupported,
    )];

    let device = Arc::new(MockDevice::new(scenario.into_iter()));
    let (protocol, controller) = Protocol::new(com_id, 0, device.clone());
    let handle = spawn(protocol.run().in_current_span());
    let result = timeout(TEST_TIMEOUT, controller.com_id_request(request)).await;
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

    let request = ComIdRequest::stack_reset(0x0005, 0x0000);

    let scenario = [
        com_id_request_event(com_id, &request),
        com_id_response_fail_event(com_id, DeviceError::NotSupported),
    ];

    let device = Arc::new(MockDevice::new(scenario.into_iter()));
    let (protocol, controller) = Protocol::new(com_id, 0, device.clone());
    let handle = spawn(protocol.run().in_current_span());
    let result = timeout(TEST_TIMEOUT, controller.com_id_request(request)).await;
    drop(controller);
    let handle_result = handle.await;

    assert_that!(result, ok(ok(err(eq(&Error::TimedOut)))));
    assert_that!(handle_result, ok(anything()));
    device.check();
}
