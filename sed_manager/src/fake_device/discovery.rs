use crate::device::Error as DeviceError;
use crate::messaging::discovery::{
    Discovery, FeatureDescriptor, LockingFeatureDescriptor, OpalV2FeatureDescriptor, OwnerPasswordState,
    TPerFeatureDescriptor,
};
use crate::serialization::{OutputStream, Serialize};

use super::route::{RouteHandler, RouteModifications};
use super::state::PersistentState;

pub struct DiscoveryHandler {}

impl RouteHandler for DiscoveryHandler {
    fn push_request(
        &mut self,
        _device: &mut PersistentState,
        _data: &[u8],
    ) -> Result<RouteModifications, crate::device::Error> {
        // Using IF-SEND on the discovery protocol and com ID is legal, but it's a noop.
        Ok(RouteModifications::none())
    }

    fn pop_response(&mut self, device: &mut PersistentState, len: usize) -> Result<Vec<u8>, crate::device::Error> {
        let discovery = Discovery::new(vec![
            get_tper_feature_desc(device),
            get_locking_feature_desc(device),
            get_ssc_feature_desc(device),
        ]);
        let mut stream = OutputStream::<u8>::new();
        discovery.serialize(&mut stream).unwrap();
        let mut buffer = stream.take();
        if buffer.len() <= len {
            buffer.resize(len, 0);
            Ok(buffer)
        } else {
            Err(DeviceError::BufferTooShort)
        }
    }
}

fn get_tper_feature_desc(_device: &PersistentState) -> FeatureDescriptor {
    // The reason this function (and below ones) take the device as parameter
    // is because if the device contains some configuration data for testing,
    // that has to be reflected in the discovery process.
    let desc = TPerFeatureDescriptor {
        sync_supported: true,
        async_supported: false,
        ack_nak_supported: false,
        buffer_mgmt_supported: false,
        streaming_supported: false,
        com_id_mgmt_supported: false,
    };
    FeatureDescriptor::TPer(desc)
}

fn get_locking_feature_desc(_device: &PersistentState) -> FeatureDescriptor {
    let desc = LockingFeatureDescriptor {
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

fn get_ssc_feature_desc(_device: &PersistentState) -> FeatureDescriptor {
    let desc = OpalV2FeatureDescriptor {
        base_com_id: 4100,
        num_com_ids: 1,
        num_locking_admins_supported: 4,
        num_locking_users_supported: 8,
        initial_owner_pw: OwnerPasswordState::SameAsMSID,
        reverted_owner_pw: OwnerPasswordState::SameAsMSID,
    };
    FeatureDescriptor::OpalV2(desc)
}
