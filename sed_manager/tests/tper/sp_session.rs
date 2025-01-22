use std::sync::Arc;

use sed_manager::fake_device::FakeDevice;
use sed_manager::messaging::types::BoolOrBytes;
use sed_manager::messaging::uid::UID;
use sed_manager::rpc::Error as RPCError;
use sed_manager::specification::sp;
use sed_manager::tper::TPer;

#[tokio::test(flavor = "multi_thread")]
async fn authenticate() -> Result<(), RPCError> {
    let device = FakeDevice::new();
    let tper = TPer::new(Arc::new(device));
    let session = tper.start_session(sp::ADMIN.try_into().unwrap()).await?;
    let result = session.authenticate(UID::new(0x0000_0009_0000_0001).try_into().unwrap(), None).await?;
    assert_eq!(result, BoolOrBytes::Bool(true));
    Ok(())
}
