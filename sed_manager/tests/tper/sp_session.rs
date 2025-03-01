use std::sync::Arc;

use sed_manager::fake_device::FakeDevice;
use sed_manager::fake_device::MSID_PASSWORD;
use sed_manager::messaging::uid::UID;
use sed_manager::rpc::Error as RPCError;
use sed_manager::rpc::MethodStatus;
use sed_manager::spec::column_types::LifeCycleState;
use sed_manager::spec::column_types::Password;
use sed_manager::spec::opal;
use sed_manager::spec::table_id;
use sed_manager::tper::TPer;

use opal::admin::sp;

#[tokio::test]
async fn authenticate_success() -> Result<(), RPCError> {
    let device = FakeDevice::new();
    let tper = TPer::new_on_default_com_id(Arc::new(device))?;
    let session = tper.start_session(sp::ADMIN, None, None).await?;
    let result = session.authenticate(opal::admin::authority::SID, Some(MSID_PASSWORD.as_bytes())).await?;
    assert_eq!(result, true);
    Ok(())
}

#[tokio::test]
async fn authenticate_wrong_password() -> Result<(), RPCError> {
    let device = FakeDevice::new();
    let tper = TPer::new_on_default_com_id(Arc::new(device))?;
    let session = tper.start_session(sp::ADMIN, None, None).await?;
    let result = session.authenticate(opal::admin::authority::SID, Some("wrong password".as_bytes())).await?;
    assert_eq!(result, false);
    Ok(())
}

#[tokio::test]
async fn authenticate_invalid_authority() -> Result<(), RPCError> {
    let device = FakeDevice::new();
    let tper = TPer::new_on_default_com_id(Arc::new(device))?;
    let session = tper.start_session(sp::ADMIN, None, None).await?;
    let invalid_authority = UID::new(0x0000_0009_2342_2342).try_into().unwrap();
    let result = session.authenticate(invalid_authority, Some(MSID_PASSWORD.as_bytes())).await;
    assert_eq!(result, Err(MethodStatus::InvalidParameter.into()));
    Ok(())
}

#[tokio::test]
async fn get_success() -> Result<(), RPCError> {
    let device = FakeDevice::new();
    let tper = TPer::new_on_default_com_id(Arc::new(device))?;
    let session = tper.start_session(sp::ADMIN, None, None).await?;
    let result = session.get::<Password>(opal::admin::c_pin::MSID.as_uid(), 3).await?;
    assert_eq!(result, MSID_PASSWORD.into());
    Ok(())
}

#[tokio::test]
async fn get_missing_object() -> Result<(), RPCError> {
    let device = FakeDevice::new();
    let tper = TPer::new_on_default_com_id(Arc::new(device))?;
    let session = tper.start_session(sp::ADMIN, None, None).await?;
    let result = session.get::<Password>(UID::new(table_id::C_PIN.as_u64() + 0x2360_4327), 3).await;
    assert_eq!(result, Err(MethodStatus::InvalidParameter.into()));
    Ok(())
}

#[tokio::test]
async fn get_invalid_column() -> Result<(), RPCError> {
    let device = FakeDevice::new();
    let tper = TPer::new_on_default_com_id(Arc::new(device))?;
    let session = tper.start_session(sp::ADMIN, None, None).await?;
    let result = session.get::<Password>(UID::new(table_id::C_PIN.as_u64() + 0x2360_4327), 57).await;
    assert_eq!(result, Err(MethodStatus::InvalidParameter.into()));
    Ok(())
}

#[tokio::test]
async fn set_success() -> Result<(), RPCError> {
    let device = FakeDevice::new();
    let tper = TPer::new_on_default_com_id(Arc::new(device))?;
    let session = tper.start_session(sp::ADMIN, None, None).await?;
    session.set(opal::admin::c_pin::SID.as_uid(), 3, Password::from("1234")).await?;
    Ok(())
}

#[tokio::test]
async fn set_missing_object() -> Result<(), RPCError> {
    let device = FakeDevice::new();
    let tper = TPer::new_on_default_com_id(Arc::new(device))?;
    let session = tper.start_session(sp::ADMIN, None, None).await?;
    let result = session.set(UID::new(table_id::C_PIN.as_u64() + 0x2360_4327), 3, Password::from("1234")).await;
    assert_eq!(result, Err(MethodStatus::InvalidParameter.into()));
    Ok(())
}

#[tokio::test]
async fn set_invalid_column() -> Result<(), RPCError> {
    let device = FakeDevice::new();
    let tper = TPer::new_on_default_com_id(Arc::new(device))?;
    let session = tper.start_session(sp::ADMIN, None, None).await?;
    let result = session.set(UID::new(table_id::C_PIN.as_u64() + 0x2360_4327), 57, Password::from("1234")).await;
    assert_eq!(result, Err(MethodStatus::InvalidParameter.into()));
    Ok(())
}

#[tokio::test]
async fn set_invalid_type() -> Result<(), RPCError> {
    let device = FakeDevice::new();
    let tper = TPer::new_on_default_com_id(Arc::new(device))?;
    let session = tper.start_session(sp::ADMIN, None, None).await?;
    let result = session.set(UID::new(table_id::C_PIN.as_u64() + 0x2360_4327), 3, 35678u32).await;
    assert_eq!(result, Err(MethodStatus::InvalidParameter.into()));
    Ok(())
}

#[tokio::test]
async fn next_success_with_uid() -> Result<(), RPCError> {
    use opal::admin::authority;
    let device = FakeDevice::new();
    let tper = TPer::new_on_default_com_id(Arc::new(device))?;
    let session = tper.start_session(sp::ADMIN, None, None).await?;
    let result = session.next(table_id::AUTHORITY, Some(authority::ADMIN.nth(1).unwrap().as_uid()), Some(2)).await?;
    assert_eq!(
        result.0,
        vec![
            authority::ADMIN.nth(2).unwrap().as_uid(),
            authority::ADMIN.nth(3).unwrap().as_uid()
        ]
    );
    Ok(())
}

#[tokio::test]
async fn next_success_no_uid() -> Result<(), RPCError> {
    let device = FakeDevice::new();
    let tper = TPer::new_on_default_com_id(Arc::new(device))?;
    let session = tper.start_session(sp::ADMIN, None, None).await?;
    let result = session.next(table_id::SP, None, None).await?;
    assert_eq!(result.0, vec![sp::ADMIN.as_uid(), sp::LOCKING.as_uid(),]);
    Ok(())
}

#[tokio::test]
async fn activate() -> Result<(), RPCError> {
    use opal::admin::authority;
    let device = Arc::new(FakeDevice::new());
    let tper = TPer::new_on_default_com_id(device.clone())?;
    let session = tper.start_session(sp::ADMIN, Some(authority::SID), Some(MSID_PASSWORD.as_bytes())).await?;
    let _ = session.activate(sp::LOCKING).await?;

    let controller = device.controller();
    let controller = controller.lock().unwrap();
    let locking_sp = controller.admin_sp.sp_specific.sp.get(&sp::LOCKING).unwrap();
    let admin1_c_pin = controller.locking_sp.basic_sp.c_pin.get(&opal::locking::c_pin::ADMIN.nth(1).unwrap()).unwrap();
    assert_eq!(locking_sp.life_cycle_state, LifeCycleState::Manufactured);
    assert_eq!(admin1_c_pin.pin.as_ref().unwrap().as_slice(), MSID_PASSWORD.as_bytes());

    Ok(())
}

#[tokio::test]
async fn revert() -> Result<(), RPCError> {
    use opal::admin::authority;
    let device = Arc::new(FakeDevice::new());

    // Activate the locking SP.
    device.controller().lock().unwrap().activate(sp::LOCKING)?;

    let tper = TPer::new_on_default_com_id(device.clone())?;
    let session = tper.start_session(sp::ADMIN, Some(authority::SID), Some(MSID_PASSWORD.as_bytes())).await?;
    let _ = session.revert(sp::LOCKING).await?;

    // Is the locking SP deactivated?
    let controller = device.controller();
    let controller = controller.lock().unwrap();
    let locking_sp = controller.admin_sp.sp_specific.sp.get(&sp::LOCKING).unwrap();
    assert_eq!(locking_sp.life_cycle_state, LifeCycleState::ManufacturedInactive);

    Ok(())
}

#[tokio::test]
async fn revert_sp() -> Result<(), RPCError> {
    use opal::locking::authority;
    let device = Arc::new(FakeDevice::new());

    // Activate the locking SP.
    device.controller().lock().unwrap().activate(sp::LOCKING)?;

    let tper = TPer::new_on_default_com_id(device.clone())?;
    let session = tper
        .start_session(sp::LOCKING, Some(authority::ADMIN.nth(1).unwrap()), Some(MSID_PASSWORD.as_bytes()))
        .await?;
    let _ = session.revert_sp(None).await?;

    // Is the locking SP deactivated?
    let controller = device.controller();
    let controller = controller.lock().unwrap();
    let locking_sp = controller.admin_sp.sp_specific.sp.get(&sp::LOCKING).unwrap();
    assert_eq!(locking_sp.life_cycle_state, LifeCycleState::ManufacturedInactive);

    Ok(())
}
