use std::sync::Arc;

use googletest::{assert_that, matchers::*};
use sed_packet::discovery::LockingDescriptor;
use sed_spec::{
    methods::{MethodStatus, Properties},
    objects::{CPin, CPinRefExt},
    preconfig::{
        core::shared::{self, method_id, table_id},
        opal_2::{
            admin::{self, sp},
            locking,
        },
    },
};
use sed_telemetry::{WithTracing, with_tracing};
use sed_tper::{Session, Tper, error::Error, protocol::Protocol};
use sed_virtual_device::{BASE_COM_ID, INITIAL_SID_PASSWORD, VirtualDevice};
use tracing::instrument;

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn session_lifetime(_with_tracing: WithTracing) {
    let device = Arc::new(VirtualDevice::new());
    let tper = Tper::connect(BASE_COM_ID, 0, device.clone(), None);
    assert!(device.sessions(BASE_COM_ID, 0).unwrap().is_empty());

    let session = tper.start_session(sp::ADMIN, None, None).await.unwrap();
    assert_eq!(device.sessions(BASE_COM_ID, 0).unwrap().len(), 1);
    session.close().await.unwrap();

    assert!(device.sessions(BASE_COM_ID, 0).unwrap().is_empty());
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn activate(_with_tracing: WithTracing) {
    let device = Arc::new(VirtualDevice::new());
    let session_id = device.insert_session(1, sp::ADMIN, Some(admin::authority::SID)).unwrap();
    let (protocol, controller) = Protocol::new(BASE_COM_ID, 0, device.clone());
    let protocol = tokio::spawn(protocol.run());

    controller.spawn(session_id, Properties::ASSUMED);
    let session = Session::from_started(sp::ADMIN, session_id, controller);
    assert_that!(session.activate(sp::LOCKING).await, ok(eq(&())));
    let _ = session.close().await;
    let _ = protocol.await;

    let discovery = device.discover();
    assert_eq!(discovery.get::<LockingDescriptor>().unwrap().locking_enabled, true);
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn authenticate(_with_tracing: WithTracing) {
    let device = Arc::new(VirtualDevice::new());
    let session_id = device.insert_session(1, sp::ADMIN, None).unwrap();
    let (protocol, controller) = Protocol::new(BASE_COM_ID, 0, device.clone());
    let protocol = tokio::spawn(protocol.run());

    controller.spawn(session_id, Properties::ASSUMED);
    let session = Session::from_started(sp::ADMIN, session_id, controller);
    assert_that!(
        session.authenticate(admin::authority::MAKERS, None).await,
        err(eq(&Error::MethodCallFailed(MethodStatus::InvalidParameter)))
    );
    assert_that!(
        session.authenticate(admin::authority::SID, None).await,
        err(eq(&Error::MethodCallFailed(MethodStatus::NotAuthorized)))
    );
    assert_that!(session.authenticate(admin::authority::SID, Some(INITIAL_SID_PASSWORD)).await, ok(eq(&())));
    let _ = session.close().await;
    let _ = protocol.await;
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn gen_key(_with_tracing: WithTracing) {
    let device = Arc::new(VirtualDevice::new());
    let session_id = device.insert_session(1, sp::LOCKING, Some(locking::authority::ADMIN.get(1).unwrap())).unwrap();
    let (protocol, controller) = Protocol::new(BASE_COM_ID, 0, device.clone());
    let protocol = tokio::spawn(protocol.run());

    controller.spawn(session_id, Properties::ASSUMED);
    let session = Session::from_started(sp::LOCKING, session_id, controller);
    assert_that!(session.gen_key(locking::k_aes_256::GLOBAL_RANGE_KEY, None, None).await, ok(eq(&())));
    let _ = session.close().await;
    let _ = protocol.await;
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn get_field(_with_tracing: WithTracing) {
    let device = Arc::new(VirtualDevice::new());
    let session_id = device.insert_session(1, sp::ADMIN, None).unwrap();
    let (protocol, controller) = Protocol::new(BASE_COM_ID, 0, device.clone());
    let protocol = tokio::spawn(protocol.run());

    controller.spawn(session_id, Properties::ASSUMED);
    let session = Session::from_started(sp::ADMIN, session_id, controller);
    assert_that!(session.get_field(admin::c_pin::MSID.pin()).await, ok(eq(&INITIAL_SID_PASSWORD)));
    let _ = session.close().await;
    let _ = protocol.await;
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn get_object(_with_tracing: WithTracing) {
    let device = Arc::new(VirtualDevice::new());
    let session_id = device.insert_session(1, sp::ADMIN, None).unwrap();
    let (protocol, controller) = Protocol::new(BASE_COM_ID, 0, device.clone());
    let protocol = tokio::spawn(protocol.run());

    controller.spawn(session_id, Properties::ASSUMED);
    let session = Session::from_started(sp::ADMIN, session_id, controller);
    let c_pin_msid = session.get_object::<CPin>(admin::c_pin::MSID, ..).await.unwrap();
    assert_that!(c_pin_msid.uid, some(eq(admin::c_pin::MSID)));
    assert_that!(c_pin_msid.pin, some(eq(&INITIAL_SID_PASSWORD)));
    let _ = session.close().await;
    let _ = protocol.await;
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn get_bytes(_with_tracing: WithTracing) {
    let device = Arc::new(VirtualDevice::new());
    let session_id = device.insert_session(1, sp::LOCKING, Some(locking::authority::ADMIN.get(0).unwrap())).unwrap();
    let (protocol, controller) = Protocol::new(BASE_COM_ID, 0, device.clone());
    let protocol = tokio::spawn(protocol.run());

    controller.spawn(session_id, Properties::ASSUMED);
    let session = Session::from_started(sp::LOCKING, session_id, controller);
    assert_that!(session.get_bytes(table_id::MBR, ..6).await, ok(len(eq(6))));
    let _ = session.close().await;
    let _ = protocol.await;
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn get_acl(_with_tracing: WithTracing) {
    let device = Arc::new(VirtualDevice::new());
    let session_id = device.insert_session(1, sp::ADMIN, None).unwrap();
    let (protocol, controller) = Protocol::new(BASE_COM_ID, 0, device.clone());
    let protocol = tokio::spawn(protocol.run());

    controller.spawn(session_id, Properties::ASSUMED);
    let session = Session::from_started(sp::ADMIN, session_id, controller);
    assert_that!(session.get_acl(admin::c_pin::MSID.into(), method_id::GET).await, ok(not(is_empty())));
    let _ = session.close().await;
    let _ = protocol.await;
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn next(_with_tracing: WithTracing) {
    let device = Arc::new(VirtualDevice::new());
    let session_id = device.insert_session(1, sp::ADMIN, None).unwrap();
    let (protocol, controller) = Protocol::new(BASE_COM_ID, 0, device.clone());
    let protocol = tokio::spawn(protocol.run());

    controller.spawn(session_id, Properties::ASSUMED);
    let session = Session::from_started(sp::ADMIN, session_id, controller);
    assert_that!(session.next(None, None).await, ok(eq(&vec![admin::sp::ADMIN, admin::sp::LOCKING])));
    assert_that!(session.next(Some(admin::sp::ADMIN), None).await, ok(eq(&vec![admin::sp::LOCKING])));
    assert_that!(session.next(Some(admin::sp::LOCKING), None).await, ok(eq(&vec![])));
    assert_that!(session.next(None, Some(1)).await, ok(eq(&vec![admin::sp::ADMIN])));
    assert_that!(session.next(Some(shared::table::C_EC_384), None).await, err(anything()));
    let _ = session.close().await;
    let _ = protocol.await;
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn revert(_with_tracing: WithTracing) {
    let device = Arc::new(VirtualDevice::new());
    let session_id = device.insert_session(1, sp::ADMIN, Some(admin::authority::SID)).unwrap();
    let (protocol, controller) = Protocol::new(BASE_COM_ID, 0, device);
    let protocol = tokio::spawn(protocol.run());

    controller.spawn(session_id, Properties::ASSUMED);
    let session = Session::from_started(sp::ADMIN, session_id, controller);
    assert_that!(session.revert(sp::LOCKING).await, ok(anything()));
    let _ = session.close().await;
    let _ = protocol.await;
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn revert_sp(_with_tracing: WithTracing) {
    let device = Arc::new(VirtualDevice::new());
    let session_id = device.insert_session(1, sp::LOCKING, Some(locking::authority::ADMIN.get(1).unwrap())).unwrap();
    let (protocol, controller) = Protocol::new(BASE_COM_ID, 0, device);
    let protocol = tokio::spawn(protocol.run());

    controller.spawn(session_id, Properties::ASSUMED);
    let session = Session::from_started(sp::LOCKING, session_id, controller);
    match session.revert_sp(None).await {
        Ok(_) => (),
        Err((session, err)) => {
            let _ = session.close().await;
            panic!("revert failed: {err}")
        }
    }
    let _ = protocol.await;
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn random(_with_tracing: WithTracing) {
    let device = Arc::new(VirtualDevice::new());
    let session_id = device.insert_session(1, sp::ADMIN, None).unwrap();
    let (protocol, controller) = Protocol::new(BASE_COM_ID, 0, device);
    let protocol = tokio::spawn(protocol.run());

    controller.spawn(session_id, Properties::ASSUMED);
    let session = Session::from_started(sp::ADMIN, session_id, controller);
    assert_that!(session.random(4).await, ok(len(eq(4))));
    let _ = session.close().await;
    let _ = protocol.await;
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn set_field(_with_tracing: WithTracing) {
    let device = Arc::new(VirtualDevice::new());
    let session_id = device.insert_session(1, sp::ADMIN, Some(admin::authority::SID)).unwrap();
    let (protocol, controller) = Protocol::new(BASE_COM_ID, 0, device.clone());
    let protocol = tokio::spawn(protocol.run());

    controller.spawn(session_id, Properties::ASSUMED);
    let session = Session::from_started(sp::ADMIN, session_id, controller);
    assert_that!(session.set_field(admin::c_pin::SID.pin(), b"asd".as_slice().into()).await, ok(anything()));
    let _ = session.close().await;
    let _ = protocol.await;
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn set_object(_with_tracing: WithTracing) {
    let device = Arc::new(VirtualDevice::new());
    let session_id = device.insert_session(1, sp::ADMIN, Some(admin::authority::SID)).unwrap();
    let (protocol, controller) = Protocol::new(BASE_COM_ID, 0, device.clone());
    let protocol = tokio::spawn(protocol.run());

    controller.spawn(session_id, Properties::ASSUMED);
    let session = Session::from_started(sp::ADMIN, session_id, controller);
    let values = CPin { pin: Some(b"asd".as_slice().into()), ..Default::default() };
    assert_that!(session.set_object(admin::c_pin::SID, values).await, ok(anything()));
    let _ = session.close().await;
    let _ = protocol.await;
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn set_bytes(_with_tracing: WithTracing) {
    let device = Arc::new(VirtualDevice::new());
    let session_id = device.insert_session(1, sp::LOCKING, Some(locking::authority::ADMIN.get(0).unwrap())).unwrap();
    let (protocol, controller) = Protocol::new(BASE_COM_ID, 0, device.clone());
    let protocol = tokio::spawn(protocol.run());

    controller.spawn(session_id, Properties::ASSUMED);
    let session = Session::from_started(sp::LOCKING, session_id, controller);
    assert_that!(session.set_bytes(table_id::MBR, 0, &[1, 2, 3]).await, ok(anything()));
    let _ = session.close().await;
    let _ = protocol.await;
}
