use crate::{
    device::{Device, Error, Interface},
    messaging::packet::{
        Discovery, FeatureDescriptor, LockingFeatureDescriptor, OpalV2FeatureDescriptor, OwnerPasswordState,
        TPerFeatureDescriptor,
    },
    serialization::{OutputStream, Serialize},
};

pub struct FakeDevice {}

#[derive(PartialEq, Eq, Hash)]
struct Route {
    protocol: u8,
    com_id: u16,
}

const DISCOVERY_ROUTING: Route = Route { protocol: 0x01, com_id: 0x0001 };

impl Device for FakeDevice {
    fn interface(&self) -> Interface {
        Interface::Other
    }
    fn model_number(&self) -> Result<String, Error> {
        Ok(String::from("FAKE-DEV-OPAL-2"))
    }
    fn serial_number(&self) -> Result<String, Error> {
        Ok(String::from("0123456789"))
    }
    fn firmware_revision(&self) -> Result<String, Error> {
        Ok(String::from("FW1.0"))
    }
    fn security_send(&self, security_protocol: u8, protocol_specific: [u8; 2], data: &[u8]) -> Result<(), Error> {
        let route = Route { protocol: security_protocol, com_id: u16::from_be_bytes(protocol_specific) };
        if route == DISCOVERY_ROUTING {
            return Ok(());
        };
        Err(Error::NotImplemented)
    }
    fn security_recv(&self, security_protocol: u8, protocol_specific: [u8; 2], len: usize) -> Result<Vec<u8>, Error> {
        let route = Route { protocol: security_protocol, com_id: u16::from_be_bytes(protocol_specific) };
        if route == DISCOVERY_ROUTING {
            let tper_desc = TPerFeatureDescriptor {
                sync_supported: true,
                async_supported: false,
                ack_nak_supported: false,
                buffer_mgmt_supported: false,
                streaming_supported: false,
                com_id_mgmt_supported: false,
            };
            let locking_desc = LockingFeatureDescriptor {
                hw_reset_supported: true,
                locked: false,
                locking_enabled: false,
                locking_supported: true,
                media_encryption: true,
                mbr_enabled: false,
                mbr_done: false,
                mbr_shadowing_not_supported: false,
            };
            let ssc_desc = OpalV2FeatureDescriptor {
                base_com_id: 4100,
                num_com_ids: 1,
                num_locking_admins_supported: 4,
                num_locking_users_supported: 8,
                initial_owner_pw: OwnerPasswordState::SameAsMSID,
                reverted_owner_pw: OwnerPasswordState::SameAsMSID,
            };
            let discovery = Discovery::new(vec![
                FeatureDescriptor::TPer(tper_desc),
                FeatureDescriptor::Locking(locking_desc),
                FeatureDescriptor::OpalV2(ssc_desc),
            ]);
            let mut stream = OutputStream::<u8>::new();
            discovery.serialize(&mut stream).unwrap();
            let mut buffer = stream.take();
            return if buffer.len() <= len {
                buffer.resize(len, 0);
                Ok(buffer)
            } else {
                Err(Error::BufferTooShort)
            };
        };
        Err(Error::NotImplemented)
    }
}
