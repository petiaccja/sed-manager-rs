use std::sync::Arc;

use googletest::{assert_that, matchers::*};
use sed_packet::discovery::LockingDescriptor;
use sed_spec::{
    methods::{MethodStatus, Properties},
    preconfig::{
        core::shared::method_id,
        opal_2::{
            admin::{self, sp},
            locking,
        },
    },
};
use sed_telemetry::{WithTracing, with_tracing};
use sed_tper::{Session, TPer, error::Error, protocol::Protocol};
use sed_virtual_device::{BASE_COM_ID, INITIAL_SID_PASSWORD, VirtualDevice};
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

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn activate(_with_tracing: WithTracing) {
    let device = Arc::new(VirtualDevice::new());
    let session_id = device.insert_session(1, sp::ADMIN, Some(admin::authority::SID)).unwrap();
    let (protocol, controller) = Protocol::new(BASE_COM_ID, 0, device.clone());
    let protocol = tokio::spawn(protocol.run());

    controller.spawn(session_id, Properties::ASSUMED);
    let session = Session::from_started(session_id, controller);
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
    let session = Session::from_started(session_id, controller);
    assert_that!(
        session.authenticate(admin::authority::MAKERS, None).await,
        err(eq(&Error::MethodCallFailed(MethodStatus::InvalidParameter)))
    );
    assert_that!(
        session.authenticate(admin::authority::SID, None).await,
        err(eq(&Error::MethodCallFailed(MethodStatus::NotAuthorized)))
    );
    assert_that!(
        session.authenticate(admin::authority::SID, Some(INITIAL_SID_PASSWORD.as_bytes().into())).await,
        ok(eq(&()))
    );
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
    let session = Session::from_started(session_id, controller);
    assert_that!(session.gen_key(locking::k_aes_256::GLOBAL_RANGE_KEY, None, None).await, ok(eq(&())));
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
    let session = Session::from_started(session_id, controller);
    assert_that!(session.get_acl(admin::c_pin::MSID.into(), method_id::GET).await, ok(not(is_empty())));
    let _ = session.close().await;
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
    let session = Session::from_started(session_id, controller);
    assert_that!(session.random(4).await, ok(len(eq(4))));
    let _ = session.close().await;
    let _ = protocol.await;
}
