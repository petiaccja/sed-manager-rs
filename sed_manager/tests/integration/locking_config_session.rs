use std::sync::Arc;

use googletest::{assert_that, matchers::*};
use sed_async::{PolyRuntime, TokioRuntime};
use sed_manager::{LockingConfigSession, SetupSession};
use sed_packet::MaxBytes;
use sed_spec::{objects::MbrControl, preconfig::opal_2::locking as opal_locking};
use sed_telemetry::{WithTracing, with_tracing};
use sed_tper::Tper;
use sed_virtual_device::{BASE_COM_ID, VirtualDevice};
use tracing::instrument;

const NEW_SID_PASSWORD: MaxBytes<32> =
    unsafe { MaxBytes::from_const_with_len_unchecked(*b"not_default\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0", 0) };

async fn setup() -> Arc<Tper> {
    let runtime = Arc::new(PolyRuntime::Tokio(TokioRuntime::current().unwrap()));
    let device = Arc::new(VirtualDevice::new());
    let tper = Arc::new(Tper::connect(BASE_COM_ID, 0, device.clone(), runtime));
    let setup_session = SetupSession::on_primary_ssc(tper.clone()).await.unwrap();

    setup_session.take_owneship(NEW_SID_PASSWORD).await.unwrap();
    setup_session.activate_secondary_sp(NEW_SID_PASSWORD).await.unwrap();

    tper
}

#[instrument]
#[rstest::rstest]
#[tokio::test]
async fn login(_with_tracing: WithTracing) {
    let tper = setup().await;

    let admin1 = opal_locking::authority::ADMIN.get(0).unwrap();
    let result = LockingConfigSession::login_on_primary_ssc(tper, admin1, Some(NEW_SID_PASSWORD)).await;
    assert_that!(result, ok(anything()));
}

#[instrument]
#[rstest::rstest]
#[tokio::test]
async fn login_wrong_password(_with_tracing: WithTracing) {
    let tper = setup().await;
    let wrong_password = MaxBytes::<32>::from(b"wrong_password".as_slice());

    let admin1 = opal_locking::authority::ADMIN.get(0).unwrap();
    let result = LockingConfigSession::login_on_primary_ssc(tper, admin1, Some(wrong_password)).await;
    assert_that!(result, err(anything()));
}

#[instrument]
#[rstest::rstest]
#[tokio::test]
async fn get_authorities(_with_tracing: WithTracing) {
    let tper = setup().await;

    let admin1 = opal_locking::authority::ADMIN.get(0).unwrap();
    let session = LockingConfigSession::login_on_primary_ssc(tper, admin1, Some(NEW_SID_PASSWORD)).await.unwrap();
    let result = session.get_authorities().await;
    assert_that!(result, ok(len(eq(15))));
}

#[instrument]
#[rstest::rstest]
#[tokio::test]
async fn get_locking_ranges(_with_tracing: WithTracing) {
    let tper = setup().await;

    let admin1 = opal_locking::authority::ADMIN.get(0).unwrap();
    let session = LockingConfigSession::login_on_primary_ssc(tper, admin1, Some(NEW_SID_PASSWORD)).await.unwrap();
    let result = session.get_locking_ranges().await;
    assert_that!(result, ok(len(eq(9))));
}

#[instrument]
#[rstest::rstest]
#[tokio::test]
async fn get_mbr(_with_tracing: WithTracing) {
    let tper = setup().await;

    let admin1 = opal_locking::authority::ADMIN.get(0).unwrap();
    let session = LockingConfigSession::login_on_primary_ssc(tper, admin1, Some(NEW_SID_PASSWORD)).await.unwrap();
    let result = session.get_mbr().await;
    assert_that!(result, ok(field!(MbrControl.enable, eq(&Some(false)))));
}
