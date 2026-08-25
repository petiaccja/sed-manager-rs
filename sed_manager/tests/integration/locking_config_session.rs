use std::sync::Arc;

use googletest::{assert_that, matchers::*};
use sed_manager::{LockingConfigSession, SetupSession};
use sed_packet::MaxBytes;
use sed_spec::preconfig::opal_2::locking as opal_locking;
use sed_telemetry::{WithTracing, with_tracing};
use sed_tper::Tper;
use sed_virtual_device::{BASE_COM_ID, VirtualDevice};
use tracing::instrument;

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn login(_with_tracing: WithTracing) {
    let new_sid_password = MaxBytes::<32>::from(b"not_default".as_slice());
    let device = Arc::new(VirtualDevice::new());
    let tper = Arc::new(Tper::connect(BASE_COM_ID, 0, device.clone(), None));
    let setup_session = SetupSession::on_primary_ssc(tper.clone()).await.unwrap();

    setup_session.take_owneship(new_sid_password.clone()).await.unwrap();
    setup_session.activate_secondary_sp(new_sid_password.clone()).await.unwrap();

    let admin1 = opal_locking::authority::ADMIN.get(0).unwrap();
    let result = LockingConfigSession::login_on_primary_ssc(tper, admin1, Some(new_sid_password)).await;
    assert_that!(result, ok(anything()));
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn login_wrong_password(_with_tracing: WithTracing) {
    let new_sid_password = MaxBytes::<32>::from(b"not_default".as_slice());
    let wrong_password = MaxBytes::<32>::from(b"wrong_password".as_slice());
    let device = Arc::new(VirtualDevice::new());
    let tper = Arc::new(Tper::connect(BASE_COM_ID, 0, device.clone(), None));
    let setup_session = SetupSession::on_primary_ssc(tper.clone()).await.unwrap();

    setup_session.take_owneship(new_sid_password.clone()).await.unwrap();
    setup_session.activate_secondary_sp(new_sid_password).await.unwrap();

    let admin1 = opal_locking::authority::ADMIN.get(0).unwrap();
    let result = LockingConfigSession::login_on_primary_ssc(tper, admin1, Some(wrong_password)).await;
    assert_that!(result, err(anything()));
}
