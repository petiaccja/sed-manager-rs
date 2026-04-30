use std::sync::Arc;

use googletest::{assert_that, matchers::*};
use sed_spec::{methods::Properties, preconfig::opal_2::admin::sp};
use sed_telemetry::{WithTracing, with_tracing};
use sed_tper::{Session, TPer, protocol::Protocol};
use sed_virtual_device::{BASE_COM_ID, VirtualDevice};
use tracing::instrument;

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn session_lifetime(_with_tracing: WithTracing) {
    let device = Arc::new(VirtualDevice::new());
    let tper = TPer::connect(BASE_COM_ID, 0, device.clone()).await;
    assert!(device.sessions(BASE_COM_ID, 0).unwrap().is_empty());

    let session = tper.start_session(sp::ADMIN, None, None).await.unwrap();
    assert_eq!(device.sessions(BASE_COM_ID, 0).unwrap().len(), 1);
    session.close().await.unwrap();

    assert!(device.sessions(BASE_COM_ID, 0).unwrap().is_empty());
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn random(_with_tracing: WithTracing) {
    let device = Arc::new(VirtualDevice::new());
    let session_id = device.start_session(1, sp::ADMIN, None);
    let (protocol, controller) = Protocol::new(BASE_COM_ID, 0, device);
    let protocol = tokio::spawn(protocol.run());

    controller.spawn(session_id, Properties::ASSUMED);
    let session = Session::from_started(session_id, controller);
    assert_that!(session.random(4).await, ok(len(eq(4))));
    let _ = session.close().await;
    let _ = protocol.await;
}
