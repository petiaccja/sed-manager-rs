use std::sync::Arc;

use sed_manager::fake_device::FakeDevice;
use sed_manager::rpc::Error as RPCError;
use sed_manager::tper::TPer;

#[tokio::test]
async fn init_stack_success() -> Result<(), RPCError> {
    let device = FakeDevice::new();
    let tper = TPer::new(Arc::new(device));

    assert_eq!(tper.com_id().await?, 4100);
    assert_eq!(tper.com_id_ext().await?, 0x0000);
    Ok(())
}
