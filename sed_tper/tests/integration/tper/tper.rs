use std::sync::Arc;

use sed_packet::{
    com_id::ComIdState,
    discovery::{GeometryDescriptor, LockingDescriptor, TperDescriptor},
};
use sed_telemetry::{WithTracing, with_tracing};
use sed_tper::{Tper, error::Error};
use sed_virtual_device::{BASE_COM_ID, VirtualDevice};
use tracing::instrument;

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn discovery(_with_tracing: WithTracing) {
    let device = VirtualDevice::new();
    let discovery = Tper::discover(&device).await.unwrap();
    assert!(discovery.get::<TperDescriptor>().is_some(), "discovery: {discovery:?}");
    assert!(discovery.get::<LockingDescriptor>().is_some(), "discovery: {discovery:?}");
    assert!(discovery.get::<GeometryDescriptor>().is_some(), "discovery: {discovery:?}");
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn verify_com_id_valid_base(_with_tracing: WithTracing) {
    let device = VirtualDevice::new();
    let tper = Tper::connect(BASE_COM_ID, 0, Arc::new(device)).await;
    let result = tper.verify_com_id_valid(BASE_COM_ID, 0).await;
    let expected = ComIdState::Issued;
    assert_eq!(result, Ok(expected));
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn verify_com_id_valid_incorrent_id(_with_tracing: WithTracing) {
    let device = VirtualDevice::new();
    let tper = Tper::connect(BASE_COM_ID, 0, Arc::new(device)).await;
    let result = tper.verify_com_id_valid(BASE_COM_ID + 1, 0).await;
    let expected = ComIdState::Inactive;
    assert_eq!(result, Ok(expected));
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn verify_com_id_valid_incorrent_ext(_with_tracing: WithTracing) {
    let device = VirtualDevice::new();
    let tper = Tper::connect(BASE_COM_ID, 0, Arc::new(device)).await;
    let result = tper.verify_com_id_valid(BASE_COM_ID, 1).await;
    let expected = ComIdState::Invalid;
    assert_eq!(result, Ok(expected));
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn stack_reset_base(_with_tracing: WithTracing) {
    let device = VirtualDevice::new();
    let tper = Tper::connect(BASE_COM_ID, 0, Arc::new(device)).await;
    let result = tper.stack_reset(BASE_COM_ID, 0).await;
    assert_eq!(result, Ok(()));
    // Do another reset just to see that the base ComId was left intact.
    let result = tper.stack_reset(BASE_COM_ID, 0).await;
    assert_eq!(result, Ok(()));
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn stack_reset_incorrent_id(_with_tracing: WithTracing) {
    let device = VirtualDevice::new();
    let tper = Tper::connect(BASE_COM_ID, 0, Arc::new(device)).await;
    let result = tper.stack_reset(BASE_COM_ID + 1, 0).await;
    assert_eq!(result, Err(Error::StackResetFailed));
}

#[instrument]
#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn stack_reset_incorrent_ext(_with_tracing: WithTracing) {
    let device = VirtualDevice::new();
    let tper = Tper::connect(BASE_COM_ID, 0, Arc::new(device)).await;
    let result = tper.stack_reset(BASE_COM_ID, 1).await;
    assert_eq!(result, Err(Error::StackResetFailed));
}
