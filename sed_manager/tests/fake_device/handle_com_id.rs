use std::sync::Arc;

use sed_manager::fake_device::FakeDevice;
use sed_manager::messaging::com_id::ComIdState;
use sed_manager::rpc::Error as RPCError;
use sed_manager::tper::TPer;

#[tokio::test]
async fn verify_com_id_associated() -> Result<(), RPCError> {
    let device = FakeDevice::new();
    let tper = TPer::new(Arc::new(device));
    let com_id = tper.com_id().await?;
    let com_id_ext = tper.com_id_ext().await?;

    assert_eq!(tper.verify_com_id(com_id, com_id_ext).await?, ComIdState::Associated);
    Ok(())
}

#[tokio::test]
async fn verify_com_id_invalid() -> Result<(), RPCError> {
    let device = FakeDevice::new();
    let tper = TPer::new(Arc::new(device));

    assert_eq!(tper.verify_com_id(0x0600, 0x0000).await?, ComIdState::Invalid);
    Ok(())
}
