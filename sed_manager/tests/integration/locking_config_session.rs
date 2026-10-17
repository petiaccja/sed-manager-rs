//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::sync::Arc;

use googletest::{assert_that, matchers::*};
use sed_async::{PolyRuntime, TokioRuntime};
use sed_manager::{Alignment, Error, LockingConfigSession, LockingRangePlanError, SetupSession, Spec};
use sed_packet::{
    MaxBytes,
    com_id::ComIdExt,
    discovery::{FeatureDescriptor, LockingDescriptor},
};
use sed_spec::{
    objects::{Authority, AuthorityRef, LockingRange, LockingRangeRef, MbrControl, MediaKeyRef},
    preconfig::{
        core::shared::mbr_control,
        opal_2::{admin::sp, locking as opal_locking},
    },
};
use sed_telemetry::with_tracing;
use sed_tper::Tper;
use sed_virtual_device::{BASE_COM_ID, LOGICAL_SECTOR_COUNT, VirtualDevice};
use tracing::instrument;

const NEW_SID_PASSWORD: MaxBytes<32> =
    unsafe { MaxBytes::from_const_with_len_unchecked(*b"not_default\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0", 0) };

async fn setup_tper() -> Tper {
    let (tper, _device) = setup().await;
    tper
}

async fn setup() -> (Tper, Arc<VirtualDevice>) {
    let runtime = Arc::new(PolyRuntime::Tokio(TokioRuntime::current().unwrap()));
    let device = Arc::new(VirtualDevice::new());
    let tper = Tper::connect(BASE_COM_ID, ComIdExt(0), device.clone(), runtime);
    let setup_session = SetupSession::new_on_primary_ssc(&tper).await.unwrap();

    setup_session.take_owneship(NEW_SID_PASSWORD).await.unwrap();
    setup_session.activate_secondary_sp(NEW_SID_PASSWORD).await.unwrap();

    (tper, device)
}

fn locking_admin1() -> AuthorityRef {
    opal_locking::authority::ADMIN.get(0).unwrap()
}

async fn login_admin1(tper: &Tper) -> Result<LockingConfigSession, Error> {
    let admin1 = locking_admin1();
    LockingConfigSession::login_on_primary_ssc(tper, admin1, Some(NEW_SID_PASSWORD)).await
}

fn range_uid(index: usize) -> LockingRangeRef {
    opal_locking::locking::RANGE.get(index).unwrap()
}

fn user_uid(index: usize) -> AuthorityRef {
    opal_locking::authority::USER.get(index).unwrap()
}

fn make_range_from_span(index: usize, start: u64, length: u64) -> LockingRange {
    LockingRange {
        uid: Some(range_uid(index)),
        range_start: Some(start),
        range_length: Some(length),
        ..Default::default()
    }
}

fn find_range_by_uid(ranges: &[LockingRange], uid: LockingRangeRef) -> &LockingRange {
    ranges.iter().find(|range| range.uid == Some(uid)).expect("locking range not found")
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn login() {
    let tper = setup_tper().await;

    let result = login_admin1(&tper).await;
    assert_that!(result, ok(anything()));
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn login_wrong_password() {
    let tper = setup_tper().await;
    let wrong_password = MaxBytes::<32>::from(b"wrong_password".as_slice());

    let admin1 = locking_admin1();
    let result = LockingConfigSession::login_on_primary_ssc(&tper, admin1, Some(wrong_password)).await;
    assert_that!(result, err(anything()));
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn get_authorities() {
    let tper = setup_tper().await;

    let session = login_admin1(&tper).await.unwrap();
    let result = session.get_authorities().await;
    assert_that!(result, ok(len(eq(15))));
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn get_locking_ranges() {
    let tper = setup_tper().await;

    let session = login_admin1(&tper).await.unwrap();
    let result = session.get_locking_ranges().await;
    assert_that!(result, ok(len(eq(9))));
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn get_mbr_size() {
    let tper = setup_tper().await;

    let session = login_admin1(&tper).await.unwrap();
    let result = session.get_mbr_size().await;
    assert_that!(result, ok(gt(&1)));
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn get_mbr_control() {
    let tper = setup_tper().await;

    let session = login_admin1(&tper).await.unwrap();
    let result = session.get_mbr_control().await;
    assert_that!(result, ok(field!(MbrControl.enable, eq(&Some(false)))));
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn get_alignment() {
    let tper = setup_tper().await;

    let session = login_admin1(&tper).await.unwrap();
    let result = session.get_alignment().await;
    assert_that!(
        result,
        ok(eq(&Alignment { alignment_required: true, alignment_granularity: 8, lowest_aligned_lba: 0 }))
    );
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn set_locking_ranges() {
    let tper = setup_tper().await;
    let session = login_admin1(&tper).await.unwrap();

    let current = session.get_locking_ranges().await.unwrap();
    let desired = [
        make_range_from_span(0, 64, 128),
        LockingRange { read_lock_enabled: Some(true), write_locked: Some(true), ..make_range_from_span(1, 256, 64) },
    ];
    let result = session.set_locking_ranges(Some(&current), &desired, LOGICAL_SECTOR_COUNT).await;
    assert_that!(result, ok(anything()));

    let updated = session.get_locking_ranges().await.unwrap();
    let range_1 = find_range_by_uid(&updated, range_uid(0));
    let range_2 = find_range_by_uid(&updated, range_uid(1));
    assert_eq!((range_1.range_start, range_1.range_length), (Some(64), Some(128)));
    assert_eq!((range_2.range_start, range_2.range_length), (Some(256), Some(64)));
    assert_eq!(range_2.read_lock_enabled, Some(true));
    assert_eq!(range_2.write_locked, Some(true));
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn set_locking_ranges_swap() {
    let tper = setup_tper().await;
    let session = login_admin1(&tper).await.unwrap();

    session
        .set_locking_ranges(
            None,
            &[
                make_range_from_span(0, 64, 64),
                make_range_from_span(1, 128, 64),
            ],
            LOGICAL_SECTOR_COUNT,
        )
        .await
        .unwrap();

    // The ranges are only allowed to overlap in the final state, not in the
    // intermediate ones, so this requires collapsing both before moving them.
    let result = session
        .set_locking_ranges(
            None,
            &[
                make_range_from_span(0, 128, 64),
                make_range_from_span(1, 64, 64),
            ],
            LOGICAL_SECTOR_COUNT,
        )
        .await;
    assert_that!(result, ok(anything()));

    let updated = session.get_locking_ranges().await.unwrap();
    let range_1 = find_range_by_uid(&updated, range_uid(0));
    let range_2 = find_range_by_uid(&updated, range_uid(1));
    assert_eq!((range_1.range_start, range_1.range_length), (Some(128), Some(64)));
    assert_eq!((range_2.range_start, range_2.range_length), (Some(64), Some(64)));
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn set_locking_ranges_stale_expected() {
    let tper = setup_tper().await;
    let session = login_admin1(&tper).await.unwrap();

    let current = session.get_locking_ranges().await.unwrap();

    let mut stale = current.clone();
    stale[3].common_name = Some("stale".into());
    let stale_uid = stale[3].uid.unwrap();

    let result = session
        .set_locking_ranges(Some(&stale), &[make_range_from_span(0, 64, 128)], LOGICAL_SECTOR_COUNT)
        .await;
    assert_that!(
        result,
        err(eq(&Error::LockingRangeUpdateRejected(LockingRangePlanError::CurrentNotMatchingExpected {
            uid: stale_uid
        })))
    );
    assert_that!(session.get_locking_ranges().await, ok(eq(&current)));
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn set_locking_ranges_invalid_desired() {
    let tper = setup_tper().await;
    let session = login_admin1(&tper).await.unwrap();

    let current = session.get_locking_ranges().await.unwrap();
    let result = session
        .set_locking_ranges(
            None,
            &[
                make_range_from_span(0, 64, 128),
                make_range_from_span(1, 128, 64),
            ],
            LOGICAL_SECTOR_COUNT,
        )
        .await;
    assert_that!(
        result,
        err(eq(&Error::LockingRangeUpdateRejected(LockingRangePlanError::OverlappingRanges {
            first: range_uid(0),
            second: range_uid(1)
        })))
    );
    assert_that!(session.get_locking_ranges().await, ok(eq(&current)));
}

fn get_media_encryption_key(virtual_device: &VirtualDevice) -> Option<[u8; 64]> {
    virtual_device.inspect(|tper| {
        let locking_sp = tper.locking_sp().unwrap();
        let key_uid = locking_sp.locking[&range_uid(2)].active_key.unwrap();
        match key_uid {
            MediaKeyRef::Aes128(_k_aes_128_ref) => panic!("no K_AES_128 table in virtual device"),
            MediaKeyRef::Aes256(k_aes_256_ref) => locking_sp.k_aes_256[&k_aes_256_ref].key,
        }
    })
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn erase_locking_range() {
    let (tper, device) = setup().await;
    let session = login_admin1(&tper).await.unwrap();

    let ranges_before = session.get_locking_ranges().await.unwrap();
    let key_before = get_media_encryption_key(&device);
    assert_that!(key_before, some(anything()));

    session.erase_locking_range(range_uid(2)).await.unwrap();

    assert_that!(get_media_encryption_key(&device), not(eq(key_before)));
    assert_that!(session.get_locking_ranges().await, ok(eq(&ranges_before)));
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn erase_locking_range_no_media_encryption() {
    let (tper, device) = setup().await;

    // Pretend the device doesn't use media encryption, like Pyrite devices.
    let mut discovery = tper.discover_current().await.unwrap();
    for feature in &mut discovery.feature_descriptors {
        if let FeatureDescriptor::Locking(LockingDescriptor { media_encryption, .. }) = feature {
            *media_encryption = false;
        }
    }
    let spec = Spec::try_from(discovery).unwrap();
    let session = LockingConfigSession::login(&tper, spec, locking_admin1(), Some(NEW_SID_PASSWORD)).await.unwrap();

    let key_before = get_media_encryption_key(&device);

    let result = session.erase_locking_range(range_uid(2)).await;

    assert_that!(result, err(eq(&Error::NotSupported)));
    assert_that!(get_media_encryption_key(&device), eq(key_before));
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn set_authorities() {
    let tper = setup_tper().await;
    let session = login_admin1(&tper).await.unwrap();

    let uid = user_uid(3);

    let current = session.get_authorities().await.unwrap();
    let desired = [Authority { uid: Some(uid), enabled: Some(true), ..Default::default() }];
    let result = session.set_authorities(Some(&current), &desired).await;
    assert_that!(result, ok(anything()));

    let updated = session.get_authorities().await.unwrap();
    let updated_auth = updated.iter().find(|authority| authority.uid == Some(uid)).unwrap();
    assert_eq!(updated_auth.enabled, Some(true));
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn change_password() {
    let tper = setup_tper().await;
    let session = login_admin1(&tper).await.unwrap();

    let authority = user_uid(3);
    let new_password = MaxBytes::<32>::from(b"wrong_password".as_slice());

    session.change_password(authority, new_password.clone()).await.unwrap();

    assert_that!(tper.start_session(sp::LOCKING, Some(authority), Some(new_password)).await, ok(anything()));
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn set_mbr_control() {
    let (tper, device) = setup().await;
    let session = login_admin1(&tper).await.unwrap();

    let mbr_control =
        MbrControl { uid: Some(mbr_control::MBR_CONTROL), enable: Some(true), done: Some(true), done_on_reset: None };

    assert_that!(
        device.inspect(|tper| tper.locking_sp().unwrap().mbr_control[&mbr_control::MBR_CONTROL].enable),
        some(eq(false))
    );

    session.set_mbr_control(mbr_control).await.unwrap();

    assert_that!(
        device.inspect(|tper| tper.locking_sp().unwrap().mbr_control[&mbr_control::MBR_CONTROL].enable),
        some(eq(true))
    );
}
