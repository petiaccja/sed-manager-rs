use std::sync::Arc;

use sed_manager::fake_device::FakeDevice;
use sed_manager::messaging::discovery::{LockingDescriptor, OpalV2Descriptor, OwnerPasswordState, TPerDescriptor};
use sed_manager::rpc::Error as RPCError;
use sed_manager::tper::TPer;

#[tokio::test]
async fn properties_none() -> Result<(), RPCError> {
    let device = FakeDevice::new();
    let tper = TPer::new(Arc::new(device));
    let (host_properties, tper_properties) = tper.properties(None).await?;
    Ok(())
}
