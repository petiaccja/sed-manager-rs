use std::{collections::BTreeSet, sync::Arc};

use googletest::{assert_that, matchers::*};
use sed_manager::{Error, SetupSession};
use sed_packet::{
    MaxBytes,
    discovery::{BlockSIDAuthDescriptor, LockingDescriptor},
};
use sed_spec::preconfig::{
    opal_2::{admin as opal_admin, locking as opal_locking},
    psid,
};
use sed_telemetry::{WithTracing, with_tracing};
use sed_tper::Tper;
use sed_virtual_device::{BASE_COM_ID, INITIAL_SID_PASSWORD, PSID_PASSWORD, VirtualDevice};
use tracing::instrument;

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn take_ownership(_with_tracing: WithTracing) {
    let device = Arc::new(VirtualDevice::new());
    let tper = Arc::new(Tper::connect(BASE_COM_ID, 0, device.clone(), None));
    let session = SetupSession::on_primary_ssc(tper).await.unwrap();

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
    let tper = Arc::new(Tper::connect(BASE_COM_ID, 0, device.clone(), None));
    let session = SetupSession::on_primary_ssc(tper).await.unwrap();

    session.take_owneship(b"not_default".as_slice().into()).await.unwrap();
    let result = session.take_owneship(b"not_default".as_slice().into()).await;
    assert_that!(result, err(eq(&Error::AlreadyOwned)));
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn activate_secondary_sp(_with_tracing: WithTracing) {
    let device = Arc::new(VirtualDevice::new());
    let tper = Arc::new(Tper::connect(BASE_COM_ID, 0, device.clone(), None));
    let session = SetupSession::on_primary_ssc(tper).await.unwrap();

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
    let tper = Arc::new(Tper::connect(BASE_COM_ID, 0, device.clone(), None));
    let session = SetupSession::on_primary_ssc(tper).await.unwrap();

    session.activate_secondary_sp(INITIAL_SID_PASSWORD).await.unwrap();
    let result = session.activate_secondary_sp(INITIAL_SID_PASSWORD).await;
    assert_that!(result, err(eq(&Error::AlreadyActivated)));
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn revert_tper_with_sid(_with_tracing: WithTracing) {
    let new_sid_password = MaxBytes::<32>::from(b"not_default".as_slice());
    let device = Arc::new(VirtualDevice::new());
    let tper = Arc::new(Tper::connect(BASE_COM_ID, 0, device.clone(), None));
    let session = SetupSession::on_primary_ssc(tper).await.unwrap();

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
    let new_sid_password = MaxBytes::<32>::from(b"not_default".as_slice());
    let device = Arc::new(VirtualDevice::new());
    let tper = Arc::new(Tper::connect(BASE_COM_ID, 0, device.clone(), None));
    let session = SetupSession::on_primary_ssc(tper).await.unwrap();

    session.take_owneship(new_sid_password.clone()).await.unwrap();
    session.activate_secondary_sp(new_sid_password).await.unwrap();

    let result = session.revert_tper(psid::admin::authority::PSID, PSID_PASSWORD).await;
    assert_that!(result, ok(anything()));

    let block_sid = device.discover().get::<BlockSIDAuthDescriptor>().unwrap().clone();
    assert_that!(block_sid.sid_msid_pin_differ, eq(false));
    let locking = device.discover().get::<LockingDescriptor>().unwrap().clone();
    assert_that!(locking.locking_enabled, eq(false));
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn revert_secondary_sp(_with_tracing: WithTracing) {
    let new_sid_password = MaxBytes::<32>::from(b"not_default".as_slice());
    let device = Arc::new(VirtualDevice::new());
    let tper = Arc::new(Tper::connect(BASE_COM_ID, 0, device.clone(), None));
    let session = SetupSession::on_primary_ssc(tper).await.unwrap();

    session.take_owneship(new_sid_password.clone()).await.unwrap();
    session.activate_secondary_sp(new_sid_password.clone()).await.unwrap();

    let result = session.revert_secondary_sp(new_sid_password).await;
    assert_that!(result, ok(anything()));

    // Secondary SP is reverted.
    let locking = device.discover().get::<LockingDescriptor>().unwrap().clone();
    assert_that!(locking.locking_enabled, eq(false));

    // Admin SP is unaffected: the changed SID password persists.
    let block_sid = device.discover().get::<BlockSIDAuthDescriptor>().unwrap().clone();
    assert_that!(block_sid.sid_msid_pin_differ, eq(true));
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn revert_secondary_sp_ex(_with_tracing: WithTracing) {
    let new_sid_password = MaxBytes::<32>::from(b"not_default".as_slice());
    let device = Arc::new(VirtualDevice::new());
    let tper = Arc::new(Tper::connect(BASE_COM_ID, 0, device.clone(), None));
    let session = SetupSession::on_primary_ssc(tper).await.unwrap();

    session.take_owneship(new_sid_password.clone()).await.unwrap();
    session.activate_secondary_sp(new_sid_password.clone()).await.unwrap();

    let admin1 = opal_locking::authority::ADMIN.get(0).unwrap();
    let result = session.revert_secondary_sp_ex(admin1, new_sid_password, None).await;
    assert_that!(result, ok(anything()));

    // Secondary SP is reverted.
    let locking = device.discover().get::<LockingDescriptor>().unwrap().clone();
    assert_that!(locking.locking_enabled, eq(false));

    // Admin SP is unaffected: the changed SID password persists.
    let block_sid = device.discover().get::<BlockSIDAuthDescriptor>().unwrap().clone();
    assert_that!(block_sid.sid_msid_pin_differ, eq(true));
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn change_password(_with_tracing: WithTracing) {
    let device = Arc::new(VirtualDevice::new());
    let tper = Arc::new(Tper::connect(BASE_COM_ID, 0, device.clone(), None));
    let session = SetupSession::on_primary_ssc(tper).await.unwrap();
    const NEW_PASSWORD: &[u8] = b"not_default".as_slice();

    // Change the password of SID, authenticating in with the MSID password.
    let result = session
        .change_password(opal_admin::sp::ADMIN, opal_admin::authority::SID, INITIAL_SID_PASSWORD, NEW_PASSWORD.into())
        .await;
    assert_that!(result, ok(eq(&())));

    // Change the password of SID again, but this time authenticate with the newly set password.
    let result = session
        .change_password(opal_admin::sp::ADMIN, opal_admin::authority::SID, NEW_PASSWORD.into(), NEW_PASSWORD.into())
        .await;
    assert_that!(result, ok(eq(&())));
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn list_authorities(_with_tracing: WithTracing) {
    let device = Arc::new(VirtualDevice::new());
    let tper = Arc::new(Tper::connect(BASE_COM_ID, 0, device.clone(), None));
    let session = SetupSession::on_primary_ssc(tper).await.unwrap();

    let result = session.list_authorities(opal_admin::sp::ADMIN).await;
    assert_that!(result, ok(anything()));
    let authorities = result.unwrap();
    let actual_uids: BTreeSet<_> = authorities.iter().filter_map(|authority| authority.uid).collect();
    let expected_uids = BTreeSet::from([
        opal_admin::authority::ANYBODY,
        opal_admin::authority::ADMINS,
        opal_admin::authority::SID,
        opal_admin::authority::MAKERS,
        opal_admin::authority::ADMIN.get(0).unwrap(),
        opal_admin::authority::ADMIN.get(1).unwrap(),
        opal_admin::authority::ADMIN.get(2).unwrap(),
        opal_admin::authority::ADMIN.get(3).unwrap(),
        psid::admin::authority::PSID,
    ]);
    assert_that!(actual_uids, eq(&expected_uids));
}
