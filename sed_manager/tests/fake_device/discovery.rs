use sed_manager::fake_device::FakeDevice;
use sed_manager::messaging::packet::{FeatureCode, FeatureDescriptor, OwnerPasswordState};
use sed_manager::tper::Error as TPerError;
use sed_manager::tper::TPer;

#[test]
fn discovery_success() -> Result<(), TPerError> {
    let device = FakeDevice {};
    let tper = TPer::new(Box::new(device));
    let discovery = tper.discovery()?;

    let Some(FeatureDescriptor::TPer(tper_desc)) = discovery.get(FeatureCode::TPer) else {
        panic!("expected a TPer feature descriptor");
    };
    let Some(FeatureDescriptor::Locking(locking_desc)) = discovery.get(FeatureCode::Locking) else {
        panic!("expected a locking feature descriptor");
    };
    let Some(FeatureDescriptor::OpalV2(opal_desc)) = discovery.get(FeatureCode::OpalV2) else {
        panic!("expected an Opal v2 feature descriptor");
    };

    assert_eq!(tper_desc.sync_supported, true);
    assert_eq!(tper_desc.async_supported, false);
    assert_eq!(tper_desc.ack_nak_supported, false);
    assert_eq!(tper_desc.buffer_mgmt_supported, false);
    assert_eq!(tper_desc.streaming_supported, false);
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
