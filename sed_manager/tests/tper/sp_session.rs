use std::sync::Arc;

use sed_manager::fake_device::FakeDevice;
use sed_manager::messaging::types::Password;
use sed_manager::messaging::uid::UID;
use sed_manager::rpc::Error as RPCError;
use sed_manager::rpc::MethodStatus;
use sed_manager::specification::opal;
use sed_manager::specification::sp;
use sed_manager::specification::table;
use sed_manager::tper::TPer;

#[tokio::test(flavor = "multi_thread")]
async fn authenticate_success() -> Result<(), RPCError> {
    let device = FakeDevice::new();
    let tper = TPer::new(Arc::new(device));
    let session = tper.start_session(sp::ADMIN.try_into().unwrap()).await?;
    let result = session
        .authenticate(opal::admin::authority::SID.try_into().unwrap(), Some("password".into()))
        .await?;
    assert_eq!(result, true);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn authenticate_wrong_password() -> Result<(), RPCError> {
    let device = FakeDevice::new();
    let tper = TPer::new(Arc::new(device));
    let session = tper.start_session(sp::ADMIN.try_into().unwrap()).await?;
    let result = session
        .authenticate(opal::admin::authority::SID.try_into().unwrap(), Some("wrong password".into()))
        .await?;
    assert_eq!(result, false);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn authenticate_invalid_authority() -> Result<(), RPCError> {
    let device = FakeDevice::new();
    let tper = TPer::new(Arc::new(device));
    let session = tper.start_session(sp::ADMIN.try_into().unwrap()).await?;
    let result = session
        .authenticate(UID::new(0x0000_0009_2342_2342).try_into().unwrap(), Some("password".into()))
        .await;
    assert_eq!(result, Err(MethodStatus::InvalidParameter.into()));
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn get_success() -> Result<(), RPCError> {
    let device = FakeDevice::new();
    let tper = TPer::new(Arc::new(device));
    let session = tper.start_session(sp::ADMIN.try_into().unwrap()).await?;
    let result = session.get::<Password>(opal::admin::c_pin::MSID, 3).await?;
    assert_eq!(result, "password".into());
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn get_missing_object() -> Result<(), RPCError> {
    let device = FakeDevice::new();
    let tper = TPer::new(Arc::new(device));
    let session = tper.start_session(sp::ADMIN.try_into().unwrap()).await?;
    let result = session.get::<Password>(UID::new(table::C_PIN.value() + 0x2360_4327), 3).await;
    assert_eq!(result, Err(MethodStatus::InvalidParameter.into()));
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn get_invalid_column() -> Result<(), RPCError> {
    let device = FakeDevice::new();
    let tper = TPer::new(Arc::new(device));
    let session = tper.start_session(sp::ADMIN.try_into().unwrap()).await?;
    let result = session.get::<Password>(UID::new(table::C_PIN.value() + 0x2360_4327), 57).await;
    assert_eq!(result, Err(MethodStatus::InvalidParameter.into()));
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn set_success() -> Result<(), RPCError> {
    let device = FakeDevice::new();
    let tper = TPer::new(Arc::new(device));
    let session = tper.start_session(sp::ADMIN.try_into().unwrap()).await?;
    session.set(opal::admin::c_pin::SID, 3, Password::from("1234")).await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn set_missing_object() -> Result<(), RPCError> {
    let device = FakeDevice::new();
    let tper = TPer::new(Arc::new(device));
    let session = tper.start_session(sp::ADMIN.try_into().unwrap()).await?;
    let result = session.set(UID::new(table::C_PIN.value() + 0x2360_4327), 3, Password::from("1234")).await;
    assert_eq!(result, Err(MethodStatus::InvalidParameter.into()));
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn set_invalid_column() -> Result<(), RPCError> {
    let device = FakeDevice::new();
    let tper = TPer::new(Arc::new(device));
    let session = tper.start_session(sp::ADMIN.try_into().unwrap()).await?;
    let result = session.set(UID::new(table::C_PIN.value() + 0x2360_4327), 57, Password::from("1234")).await;
    assert_eq!(result, Err(MethodStatus::InvalidParameter.into()));
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn set_invalid_type() -> Result<(), RPCError> {
    let device = FakeDevice::new();
    let tper = TPer::new(Arc::new(device));
    let session = tper.start_session(sp::ADMIN.try_into().unwrap()).await?;
    let result = session.set(UID::new(table::C_PIN.value() + 0x2360_4327), 3, 35678u32).await;
    assert_eq!(result, Err(MethodStatus::InvalidParameter.into()));
    Ok(())
}
