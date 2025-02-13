use crate::messaging::discovery::{
    Discovery, FeatureDescriptor, LockingDescriptor, OpalV2Descriptor, OwnerPasswordState, TPerDescriptor,
};
use crate::rpc::Properties;
use crate::serialization::{OutputStream, Serialize};

pub const BASE_COM_ID: u16 = 4100;
pub const NUM_COM_IDS: u16 = 1;

pub fn write_discovery(discovery: &Discovery, len: usize) -> Result<Vec<u8>, crate::device::Error> {
    let mut stream = OutputStream::<u8>::new();
    discovery.serialize(&mut stream).unwrap();
    let mut buffer = stream.take();
    buffer.resize(len, 0); // If the transfer length is too small, the truncated buffer must be returned.
    Ok(buffer)
}

pub fn get_discovery(properties: Properties) -> Discovery {
    Discovery::new(vec![
        get_tper_feature_desc(properties),
        get_locking_feature_desc(),
        get_ssc_feature_desc(),
    ])
}

fn get_tper_feature_desc(properties: Properties) -> FeatureDescriptor {
    let desc = TPerDescriptor {
        sync_supported: true,
        async_supported: properties.asynchronous,
        ack_nak_supported: properties.ack_nak,
        buffer_mgmt_supported: properties.buffer_mgmt,
        streaming_supported: true,
        com_id_mgmt_supported: false,
    };
    FeatureDescriptor::TPer(desc)
}

fn get_locking_feature_desc() -> FeatureDescriptor {
    let desc = LockingDescriptor {
        hw_reset_supported: true,
        locked: false,
        locking_enabled: false,
        locking_supported: true,
        media_encryption: true,
        mbr_enabled: false,
        mbr_done: false,
        mbr_shadowing_not_supported: false,
    };
    FeatureDescriptor::Locking(desc)
}

fn get_ssc_feature_desc() -> FeatureDescriptor {
    let desc = OpalV2Descriptor {
        base_com_id: BASE_COM_ID,
        num_com_ids: NUM_COM_IDS,
        no_range_crossing: false,
        num_locking_admins_supported: 4,
        num_locking_users_supported: 8,
        initial_owner_pw: OwnerPasswordState::SameAsMSID,
        reverted_owner_pw: OwnerPasswordState::SameAsMSID,
    };
    FeatureDescriptor::OpalV2(desc)
}
