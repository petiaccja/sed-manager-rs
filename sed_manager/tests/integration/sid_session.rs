use std::sync::Arc;

use googletest::{assert_that, matchers::*};
use sed_manager::{error::Error, sid_session::SidSession};
use sed_packet::discovery::{BlockSIDAuthDescriptor, LockingDescriptor};
use sed_spec::preconfig::{opal_2::admin as opal_admin, psid};
use sed_telemetry::{WithTracing, with_tracing};
use sed_tper::Tper;
use sed_virtual_device::{BASE_COM_ID, INITIAL_SID_PASSWORD, PSID_PASSWORD, VirtualDevice};
use tracing::instrument;

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn take_ownership(_with_tracing: WithTracing) {
    let device = Arc::new(VirtualDevice::new());
    let tper = Arc::new(Tper::connect(BASE_COM_ID, 0, device.clone()).await);
    let session = SidSession::on_primary_ssc(tper).await.unwrap();

    let block_sid_before = device.discover().get::<BlockSIDAuthDescriptor>().unwrap().clone();
    assert_that!(block_sid_before.sid_msid_pin_differ, eq(false));

    let result = session.take_owneship(b"not_default".as_slice().into()).await;
    assert_that!(result, ok(anything()));

    let block_sid_after = device.discover().get::<BlockSIDAuthDescriptor>().unwrap().clone();
    assert_that!(block_sid_after.sid_msid_pin_differ, eq(true));
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn take_ownership_already_owned(_with_tracing: WithTracing) {
    let device = Arc::new(VirtualDevice::new());
    let tper = Arc::new(Tper::connect(BASE_COM_ID, 0, device.clone()).await);
    let session = SidSession::on_primary_ssc(tper).await.unwrap();

    session.take_owneship(b"not_default".as_slice().into()).await.unwrap();
    let result = session.take_owneship(b"not_default".as_slice().into()).await;
    assert_that!(result, err(eq(&Error::AlreadyOwned)));
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn activate_secondary_sp(_with_tracing: WithTracing) {
    let device = Arc::new(VirtualDevice::new());
    let tper = Arc::new(Tper::connect(BASE_COM_ID, 0, device.clone()).await);
    let session = SidSession::on_primary_ssc(tper).await.unwrap();

    let locking_before = device.discover().get::<LockingDescriptor>().unwrap().clone();
    assert_that!(locking_before.locking_enabled, eq(false));

    let result = session.activate_secondary_sp(INITIAL_SID_PASSWORD).await;
    assert_that!(result, ok(anything()));

    let locking_after = device.discover().get::<LockingDescriptor>().unwrap().clone();
    assert_that!(locking_after.locking_enabled, eq(true));
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn activate_secondary_sp_already_activated(_with_tracing: WithTracing) {
    let device = Arc::new(VirtualDevice::new());
    let tper = Arc::new(Tper::connect(BASE_COM_ID, 0, device.clone()).await);
    let session = SidSession::on_primary_ssc(tper).await.unwrap();

    session.activate_secondary_sp(INITIAL_SID_PASSWORD).await.unwrap();
    let result = session.activate_secondary_sp(INITIAL_SID_PASSWORD).await;
    assert_that!(result, err(eq(&Error::AlreadyActivated)));
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn revert_tper_with_sid(_with_tracing: WithTracing) {
    let new_sid_password: sed_packet::MaxBytes<32> = b"not_default".as_slice().into();
    let device = Arc::new(VirtualDevice::new());
    let tper = Arc::new(Tper::connect(BASE_COM_ID, 0, device.clone()).await);
    let session = SidSession::on_primary_ssc(tper).await.unwrap();
    session.take_owneship(new_sid_password.clone()).await.unwrap();
    session.activate_secondary_sp(new_sid_password.clone()).await.unwrap();

    let result = session.revert_tper(opal_admin::authority::SID, new_sid_password).await;
    assert_that!(result, ok(anything()));

    let block_sid = device.discover().get::<BlockSIDAuthDescriptor>().unwrap().clone();
    assert_that!(block_sid.sid_msid_pin_differ, eq(false));
    let locking = device.discover().get::<LockingDescriptor>().unwrap().clone();
    assert_that!(locking.locking_enabled, eq(false));
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn revert_tper_with_psid(_with_tracing: WithTracing) {
    let new_sid_password: sed_packet::MaxBytes<32> = b"not_default".as_slice().into();
    let device = Arc::new(VirtualDevice::new());
    let tper = Arc::new(Tper::connect(BASE_COM_ID, 0, device.clone()).await);
    let session = SidSession::on_primary_ssc(tper).await.unwrap();
    session.take_owneship(new_sid_password.clone()).await.unwrap();
    session.activate_secondary_sp(new_sid_password).await.unwrap();

    let result = session.revert_tper(psid::admin::authority::PSID, PSID_PASSWORD).await;
    assert_that!(result, ok(anything()));

    let block_sid = device.discover().get::<BlockSIDAuthDescriptor>().unwrap().clone();
    assert_that!(block_sid.sid_msid_pin_differ, eq(false));
    let locking = device.discover().get::<LockingDescriptor>().unwrap().clone();
    assert_that!(locking.locking_enabled, eq(false));
}
