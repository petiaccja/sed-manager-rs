use std::sync::Arc;
use std::time::Duration;

use googletest::{assert_that, matchers::*};
use tokio::spawn;
use tracing::{Instrument, instrument};

use sed_device::mock_device::MockDevice;
use sed_telemetry::{WithTracing, with_tracing};
use sed_tper::protocol::Protocol;

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn shutdown(_with_tracing: WithTracing) {
    let device = Arc::new(MockDevice::new([].into_iter()));
    let (protocol, _) = Protocol::new(1, 0, device);
    let handle = spawn(protocol.run().in_current_span());
    assert_that!(tokio::time::timeout(Duration::from_secs(5), handle).await, ok(anything()));
}
