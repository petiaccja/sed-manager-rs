//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::sync::Arc;

use googletest::{assert_that, matchers::*};
use sed_async::{PolyRuntime, TokioRuntime};
use sed_packet::{
    com_id::ComIdExt,
    discovery::LockingDescriptor,
    token_stream::token::{Atom as _, MediumAtom},
};
use sed_spec::{
    methods::{GetBytesResult, MethodStatus, Properties, SessionProperties, SetBytes},
    objects::{AuthorityRef, CPin, CPinRefExt, SecurityProviderRef},
    preconfig::{
        core::shared::{self, method_id, table_id},
        opal_2::{
            admin::{self, sp},
            locking,
        },
    },
};
use sed_telemetry::with_tracing;
use sed_tper::{Error, Session, Tper, protocol::Protocol};
use sed_virtual_device::{BASE_COM_ID, INITIAL_SID_PASSWORD, VirtualDevice};
use tokio::task::JoinHandle;
use tracing::instrument;

/// Start a protocol stack for the `device`, and spawn a session to the `sp`
/// on both ends.
///
/// The session is inserted into the device and the protocol stack directly,
/// bypassing `StartSession`, so tests can focus on the session's methods.
fn spawn_session(
    device: &Arc<VirtualDevice>,
    sp: SecurityProviderRef,
    authority: Option<AuthorityRef>,
) -> (Session, JoinHandle<()>) {
    let runtime = Arc::new(PolyRuntime::Tokio(TokioRuntime::current().unwrap()));
    let session_id = device.insert_session(1, sp, authority).unwrap();
    let (protocol, controller) = Protocol::new(BASE_COM_ID, ComIdExt(0), None, device.clone(), runtime, false);
    let protocol = tokio::spawn(protocol.run());

    controller.spawn(session_id, SessionProperties::INITIAL_OPAL_FAMILY);
    let session = Session::from_started(sp, session_id, SessionProperties::INITIAL_OPAL_FAMILY, controller);
    (session, protocol)
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn session_lifetime() {
    let device = Arc::new(VirtualDevice::default());
    let runtime = Arc::new(PolyRuntime::Tokio(TokioRuntime::current().unwrap()));
    let tper = Tper::connect(BASE_COM_ID, ComIdExt(0), None, device.clone(), runtime);
    assert!(device.sessions(BASE_COM_ID, ComIdExt(0)).unwrap().is_empty());

    let session = tper.start_session(sp::ADMIN, None, None).await.unwrap();
    assert_eq!(device.sessions(BASE_COM_ID, ComIdExt(0)).unwrap().len(), 1);
    session.close().await.unwrap();

    assert!(device.sessions(BASE_COM_ID, ComIdExt(0)).unwrap().is_empty());
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn activate() {
    let device = Arc::new(VirtualDevice::default());
    let (session, protocol) = spawn_session(&device, sp::ADMIN, Some(admin::authority::SID));
    assert_that!(session.activate(sp::LOCKING).await, ok(eq(&())));
    let _ = session.close().await;
    let _ = protocol.await;

    let discovery = device.discover();
    assert_eq!(discovery.get::<LockingDescriptor>().unwrap().locking_enabled, true);
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn authenticate() {
    let device = Arc::new(VirtualDevice::default());
    let (session, protocol) = spawn_session(&device, sp::ADMIN, None);
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
#[tokio::test]
#[with_tracing]
async fn gen_key() {
    let device = Arc::new(VirtualDevice::default());
    let (session, protocol) = spawn_session(&device, sp::LOCKING, Some(locking::authority::ADMIN.get(1).unwrap()));
    assert_that!(session.gen_key(locking::k_aes_256::GLOBAL_RANGE_KEY, None, None).await, ok(eq(&())));
    let _ = session.close().await;
    let _ = protocol.await;
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn get_field() {
    let device = Arc::new(VirtualDevice::default());
    let (session, protocol) = spawn_session(&device, sp::ADMIN, None);
    assert_that!(session.get_field(admin::c_pin::MSID.pin()).await, ok(eq(&INITIAL_SID_PASSWORD)));
    let _ = session.close().await;
    let _ = protocol.await;
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn get_object() {
    let device = Arc::new(VirtualDevice::default());
    let (session, protocol) = spawn_session(&device, sp::ADMIN, None);
    let c_pin_msid = session.get_object::<CPin>(admin::c_pin::MSID, ..).await.unwrap();
    assert_that!(c_pin_msid.uid, some(eq(admin::c_pin::MSID)));
    assert_that!(c_pin_msid.pin, some(eq(&INITIAL_SID_PASSWORD)));
    let _ = session.close().await;
    let _ = protocol.await;
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn get_bytes() {
    let device = Arc::new(VirtualDevice::default());
    let (session, protocol) = spawn_session(&device, sp::LOCKING, Some(locking::authority::ADMIN.get(0).unwrap()));
    assert_that!(session.get_bytes(table_id::MBR, ..6).await, ok(len(eq(6))));
    let _ = session.close().await;
    let _ = protocol.await;
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn get_bytes_oversize() {
    let device = Arc::new(VirtualDevice::default());
    let (session, protocol) = spawn_session(&device, sp::LOCKING, Some(locking::authority::ADMIN.get(0).unwrap()));

    let max_payload_len = max_get_payload(&session.properties().inbound) as u64;

    let result = session.get_bytes(table_id::MBR, 0..(max_payload_len + 1)).await;
    assert_that!(result, err(eq(&Error::MethodCallFailed(MethodStatus::ResponseOverflow))));
    let _ = session.close().await;
    let _ = protocol.await;
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn get_bytes_limit_size() {
    let device = Arc::new(VirtualDevice::default());
    let (session, protocol) = spawn_session(&device, sp::LOCKING, Some(locking::authority::ADMIN.get(0).unwrap()));

    let max_payload_len = max_get_payload(&session.properties().inbound) as u64;

    let result = session.get_bytes(table_id::MBR, 0..(max_payload_len)).await;
    assert_that!(result, ok(len(eq(max_payload_len as usize))));
    let _ = session.close().await;
    let _ = protocol.await;
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn get_acl() {
    let device = Arc::new(VirtualDevice::default());
    let (session, protocol) = spawn_session(&device, sp::ADMIN, None);
    assert_that!(session.get_acl(admin::c_pin::MSID.into(), method_id::GET).await, ok(not(is_empty())));
    let _ = session.close().await;
    let _ = protocol.await;
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn next() {
    let device = Arc::new(VirtualDevice::default());
    let (session, protocol) = spawn_session(&device, sp::ADMIN, None);
    assert_that!(session.next(None, None).await, ok(eq(&vec![admin::sp::ADMIN, admin::sp::LOCKING])));
    assert_that!(session.next(Some(admin::sp::ADMIN), None).await, ok(eq(&vec![admin::sp::LOCKING])));
    assert_that!(session.next(Some(admin::sp::LOCKING), None).await, ok(eq(&vec![])));
    assert_that!(session.next(None, Some(1)).await, ok(eq(&vec![admin::sp::ADMIN])));
    assert_that!(session.next(Some(shared::table::C_EC_384), None).await, err(anything()));
    let _ = session.close().await;
    let _ = protocol.await;
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn revert() {
    let device = Arc::new(VirtualDevice::default());
    let (session, protocol) = spawn_session(&device, sp::ADMIN, Some(admin::authority::SID));
    assert_that!(session.revert(sp::LOCKING).await, ok(anything()));
    let _ = session.close().await;
    let _ = protocol.await;
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn revert_sp() {
    let device = Arc::new(VirtualDevice::default());
    let (session, protocol) = spawn_session(&device, sp::LOCKING, Some(locking::authority::ADMIN.get(1).unwrap()));
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
#[tokio::test]
#[with_tracing]
async fn random() {
    let device = Arc::new(VirtualDevice::default());
    let (session, protocol) = spawn_session(&device, sp::ADMIN, None);
    assert_that!(session.random(4).await, ok(len(eq(4))));
    let _ = session.close().await;
    let _ = protocol.await;
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn set_field() {
    let device = Arc::new(VirtualDevice::default());
    let (session, protocol) = spawn_session(&device, sp::ADMIN, Some(admin::authority::SID));
    assert_that!(session.set_field(admin::c_pin::SID.pin(), b"asd".as_slice().into()).await, ok(anything()));
    let _ = session.close().await;
    let _ = protocol.await;
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn set_object() {
    let device = Arc::new(VirtualDevice::default());
    let (session, protocol) = spawn_session(&device, sp::ADMIN, Some(admin::authority::SID));
    let values = CPin { pin: Some(b"asd".as_slice().into()), ..Default::default() };
    assert_that!(session.set_object(admin::c_pin::SID, values).await, ok(anything()));
    let _ = session.close().await;
    let _ = protocol.await;
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn set_bytes() {
    let device = Arc::new(VirtualDevice::default());
    let (session, protocol) = spawn_session(&device, sp::LOCKING, Some(locking::authority::ADMIN.get(0).unwrap()));
    assert_that!(session.set_bytes(table_id::MBR, 234, &[1, 2, 3]).await, ok(anything()));
    let _ = session.close().await;
    let _ = protocol.await;
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn set_bytes_oversize() {
    let device = Arc::new(VirtualDevice::default());
    let (session, protocol) = spawn_session(&device, sp::LOCKING, Some(locking::authority::ADMIN.get(0).unwrap()));

    let max_method_len = session.properties().outbound.max_method_len();
    let max_payload = max_set_payload(&session.properties().outbound);
    let payload = vec![0; max_payload + 1];

    assert_that!(
        session.set_bytes(table_id::MBR, 0, &payload).await,
        err(eq(&Error::MethodTooLarge { requested: max_method_len + 1, maximum: max_method_len }))
    );
    let _ = session.close().await;
    let _ = protocol.await;
}

#[instrument]
#[tokio::test]
#[with_tracing]
async fn set_bytes_limit_size() {
    let device = Arc::new(VirtualDevice::default());
    let (session, protocol) = spawn_session(&device, sp::LOCKING, Some(locking::authority::ADMIN.get(0).unwrap()));

    let max_payload = max_set_payload(&session.properties().outbound);
    let payload = vec![0; max_payload];

    assert_that!(session.set_bytes(table_id::MBR, 0, &payload).await, ok(anything()));
    let _ = session.close().await;
    let _ = protocol.await;
}

/// [`SetBytes::HEADER_LEN`] assumes an 8-byte `where` argument, but we
/// only use zero bytes (encoded in tiny atom header).
const WHERE_ARG_COMPENSATION: usize = 8;

fn max_get_payload(inbound_properties: &Properties) -> usize {
    // The initial properties will always use a medium atom (len: 16..2047).
    let max_method_len = inbound_properties.max_method_len();
    max_method_len - GetBytesResult::HEADER_LEN - MediumAtom::HEADER_LEN
}

fn max_set_payload(outbound_properties: &Properties) -> usize {
    // The initial properties will always use a medium atom (len: 16..2047).
    let max_method_len = outbound_properties.max_method_len();
    max_method_len - SetBytes::HEADER_LEN - MediumAtom::HEADER_LEN + WHERE_ARG_COMPENSATION
}
