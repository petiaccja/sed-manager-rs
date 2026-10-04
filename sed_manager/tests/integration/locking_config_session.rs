//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::sync::Arc;

use googletest::{assert_that, matchers::*};
use sed_async::{PolyRuntime, TokioRuntime};
use sed_manager::{Alignment, LockingConfigSession, SetupSession};
use sed_packet::{MaxBytes, com_id::ComIdExt};
use sed_spec::{objects::MbrControl, preconfig::opal_2::locking as opal_locking};
use sed_telemetry::with_tracing;
use sed_tper::Tper;
use sed_virtual_device::{BASE_COM_ID, VirtualDevice};
use tracing::instrument;

const NEW_SID_PASSWORD: MaxBytes<32> =
    unsafe { MaxBytes::from_const_with_len_unchecked(*b"not_default\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0", 0) };

async fn setup() -> Tper {
    let runtime = Arc::new(PolyRuntime::Tokio(TokioRuntime::current().unwrap()));
    let device = Arc::new(VirtualDevice::new());
    let tper = Tper::connect(BASE_COM_ID, ComIdExt(0), device.clone(), runtime);
    let setup_session = SetupSession::new_on_primary_ssc(&tper).await.unwrap();

    setup_session.take_owneship(NEW_SID_PASSWORD).await.unwrap();
    setup_session.activate_secondary_sp(NEW_SID_PASSWORD).await.unwrap();

    tper
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn login() {
    let tper = setup().await;

    let admin1 = opal_locking::authority::ADMIN.get(0).unwrap();
    let result = LockingConfigSession::login_on_primary_ssc(&tper, admin1, Some(NEW_SID_PASSWORD)).await;
    assert_that!(result, ok(anything()));
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn login_wrong_password() {
    let tper = setup().await;
    let wrong_password = MaxBytes::<32>::from(b"wrong_password".as_slice());

    let admin1 = opal_locking::authority::ADMIN.get(0).unwrap();
    let result = LockingConfigSession::login_on_primary_ssc(&tper, admin1, Some(wrong_password)).await;
    assert_that!(result, err(anything()));
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn get_authorities() {
    let tper = setup().await;

    let admin1 = opal_locking::authority::ADMIN.get(0).unwrap();
    let session = LockingConfigSession::login_on_primary_ssc(&tper, admin1, Some(NEW_SID_PASSWORD)).await.unwrap();
    let result = session.get_authorities().await;
    assert_that!(result, ok(len(eq(15))));
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn get_locking_ranges() {
    let tper = setup().await;

    let admin1 = opal_locking::authority::ADMIN.get(0).unwrap();
    let session = LockingConfigSession::login_on_primary_ssc(&tper, admin1, Some(NEW_SID_PASSWORD)).await.unwrap();
    let result = session.get_locking_ranges().await;
    assert_that!(result, ok(len(eq(9))));
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn get_mbr_size() {
    let tper = setup().await;

    let admin1 = opal_locking::authority::ADMIN.get(0).unwrap();
    let session = LockingConfigSession::login_on_primary_ssc(&tper, admin1, Some(NEW_SID_PASSWORD)).await.unwrap();
    let result = session.get_mbr_size().await;
    assert_that!(result, ok(gt(&1)));
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn get_mbr_control() {
    let tper = setup().await;

    let admin1 = opal_locking::authority::ADMIN.get(0).unwrap();
    let session = LockingConfigSession::login_on_primary_ssc(&tper, admin1, Some(NEW_SID_PASSWORD)).await.unwrap();
    let result = session.get_mbr_control().await;
    assert_that!(result, ok(field!(MbrControl.enable, eq(&Some(false)))));
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn get_alignment() {
    let tper = setup().await;

    let admin1 = opal_locking::authority::ADMIN.get(0).unwrap();
    let session = LockingConfigSession::login_on_primary_ssc(&tper, admin1, Some(NEW_SID_PASSWORD)).await.unwrap();
    let result = session.get_alignment().await;
    assert_that!(
        result,
        ok(eq(&Alignment { alignment_required: true, alignment_granularity: 8, lowest_aligned_lba: 0 }))
    );
}
