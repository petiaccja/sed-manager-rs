//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::sync::Arc;

use sed_manager::fake_device::FakeDevice;
use sed_manager::rpc::{Error as RPCError, Properties, TokioRuntime};
use sed_manager::spec::opal;
use sed_manager::tper::TPer;

#[tokio::test]
async fn properties_with_host() -> Result<(), RPCError> {
    let runtime = Arc::new(TokioRuntime::new());
    let device = Arc::new(FakeDevice::new());
    let device_caps = device.capabilities().clone();
    let tper = TPer::new_on_default_com_id(device, runtime)?;
    let tper_caps = tper.capabilities();
    let new_properties = tper.change_properties(&tper_caps).await;
    assert_eq!(new_properties, Properties::common(&device_caps, &tper_caps));
    assert_eq!(tper.current_properties().await, new_properties);
    Ok(())
}

#[tokio::test]
async fn start_session_normal() -> Result<(), RPCError> {
    let runtime = Arc::new(TokioRuntime::new());
    let device = Arc::new(FakeDevice::new());
    {
        let tper = TPer::new_on_default_com_id(device.clone(), runtime)?;
        let session = tper.start_session(opal::admin::sp::ADMIN.into(), None, None).await?;
        session.end_session().await?;
    }
    assert!(device.active_sessions().is_empty());
    Ok(())
}
