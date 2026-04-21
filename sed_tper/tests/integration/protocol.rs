use std::sync::Arc;
use std::time::Duration;

use googletest::{assert_that, matchers::*};
use sed_packet::{
    packet::{ComPacket, Packet, SubPacket, SubPacketKind},
    token::ToTokens,
};
use sed_spec::{
    methods::{MethodCall, MethodStatus, StartSession, SyncSession},
    preconfig::{
        core::shared::{
            invoking_id::SESSION_MANAGER,
            sm_method_id::{START_SESSION, SYNC_SESSION},
        },
        opal_2::admin::sp,
    },
};
use sorbit::ser_de::ToBytes as _;
use tracing::{Instrument, instrument};

use sed_device::mock_device::{MockDevice, MockEvent};
use sed_telemetry::{WithTracing, with_tracing};
use sed_tper::protocol::Protocol;

const TIMEOUT: Duration = Duration::from_secs(5);

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn shutdown(_with_tracing: WithTracing) {
    println!("bitch");
    let device = Arc::new(MockDevice::new([].into_iter()));
    let (protocol, controller) = Protocol::new(1, 0, device);
    let handle = tokio::spawn(protocol.run().in_current_span());
    assert!(controller.shutdown(TIMEOUT).await.is_ok());
    assert_that!(tokio::time::timeout(Duration::from_secs(5), handle).await, ok(anything()));
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn send_management_method(_with_tracing: WithTracing) {
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
    let handle = tokio::spawn(protocol.run().in_current_span());
    let result = tokio::time::timeout(TIMEOUT, controller.call(None, start_session.to_tokens().unwrap())).await;
    let shutdown_result = controller.shutdown(TIMEOUT).await;
    let _ = handle.await;

    assert_that!(result, ok(ok(ok(eq(&sync_session.to_tokens().unwrap())))));
    assert_that!(shutdown_result, ok(anything()));
    device.check();
}

fn method_call_event(com_id: u16, hsn: u32, tsn: u32, call: impl ToTokens) -> MockEvent {
    MockEvent::Send {
        name: Some("method_call".into()),
        security_protocol: 0x01,
        protocol_specific: com_id.to_be_bytes(),
        expected: packetize(com_id, hsn, tsn, call).to_bytes().unwrap(),
        result: Ok(()),
    }
}

fn method_return_event(com_id: u16, hsn: u32, tsn: u32, return_: impl ToTokens) -> MockEvent {
    MockEvent::Recv {
        name: Some("method_return".into()),
        security_protocol: 0x01,
        protocol_specific: com_id.to_be_bytes(),
        result: Ok(packetize(com_id, hsn, tsn, return_).to_bytes().unwrap()),
    }
}

fn packetize(com_id: u16, hsn: u32, tsn: u32, value: impl ToTokens) -> ComPacket {
    ComPacket {
        com_id,
        com_id_ext: 0,
        payload: vec![Packet {
            tper_session_number: tsn,
            host_session_number: hsn,
            payload: vec![SubPacket {
                kind: SubPacketKind::Data,
                length: std::marker::PhantomData,
                payload: value.to_tokens().unwrap(),
            }],
            ..Default::default()
        }],
        ..Default::default()
    }
}
