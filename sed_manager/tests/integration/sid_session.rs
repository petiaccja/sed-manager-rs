use std::sync::Arc;

use googletest::{assert_that, matchers::*};
use sed_manager::sid_session::SidSession;
use sed_packet::discovery::BlockSIDAuthDescriptor;
use sed_telemetry::{WithTracing, with_tracing};
use sed_tper::Tper;
use sed_virtual_device::{BASE_COM_ID, VirtualDevice};
use tracing::instrument;

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn take_ownership(_with_tracing: WithTracing) {
    let device = Arc::new(VirtualDevice::new());
    let tper = Arc::new(Tper::connect(BASE_COM_ID, 0, device.clone()).await);
    let session = SidSession::on_primary_ssc(tper).await.unwrap();
    let result = session.take_owneship(b"not_default".as_slice().into()).await;
    assert_that!(result, ok(anything()));
    let block_sid_desc = device.discover().get::<BlockSIDAuthDescriptor>().unwrap().clone();
    assert_that!(block_sid_desc.sid_msid_pin_differ, eq(true));
}
