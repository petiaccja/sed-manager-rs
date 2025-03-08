use sed_manager::fake_device::FakeDevice;
use sed_manager::messaging::discovery::{LockingDescriptor, OpalV2Descriptor, OwnerPasswordState, TPerDescriptor};
use sed_manager::rpc::Error as RPCError;
use sed_manager::tper::discover;

#[test]
fn discovery_normal() -> Result<(), RPCError> {
    let device = FakeDevice::new();
    let discovery = discover(&device)?;

    let Some(tper_desc) = discovery.get::<TPerDescriptor>() else {
        panic!("expected a TPer feature descriptor");
    };
    let Some(locking_desc) = discovery.get::<LockingDescriptor>() else {
        panic!("expected a locking feature descriptor");
    };
    let Some(opal_desc) = discovery.get::<OpalV2Descriptor>() else {
        panic!("expected an Opal v2 feature descriptor");
    };

    assert_eq!(tper_desc.sync_supported, true);
    assert_eq!(tper_desc.async_supported, true);
    assert_eq!(tper_desc.ack_nak_supported, false);
    assert_eq!(tper_desc.buffer_mgmt_supported, false);
    assert_eq!(tper_desc.streaming_supported, true);
    assert_eq!(tper_desc.com_id_mgmt_supported, false);

    assert_eq!(locking_desc.locking_supported, true);
    assert_eq!(locking_desc.locking_enabled, false);
    assert_eq!(locking_desc.locked, false);
    assert_eq!(locking_desc.media_encryption, false);
    assert_eq!(locking_desc.mbr_enabled, false);
    assert_eq!(locking_desc.mbr_done, false);
    assert_eq!(locking_desc.mbr_shadowing_not_supported, false);
    assert_eq!(locking_desc.hw_reset_supported, true);

    assert_eq!(opal_desc.base_com_id, 4100);
    assert_eq!(opal_desc.num_com_ids, 1);
    assert_eq!(opal_desc.num_locking_admins_supported, 4);
    assert_eq!(opal_desc.num_locking_users_supported, 8);
    assert_eq!(opal_desc.initial_owner_pw, OwnerPasswordState::SameAsMSID);
    assert_eq!(opal_desc.reverted_owner_pw, OwnerPasswordState::SameAsMSID);

    Ok(())
}
