use std::sync::Arc;

use sed_spec::preconfig::opal_2::admin::sp;
use sed_telemetry::{WithTracing, with_tracing};
use sed_tper::TPer;
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
