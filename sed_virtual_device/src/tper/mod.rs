mod admin;
mod locking;
mod opal_2;
mod preconfig_shared;
mod security_provider;

use std::marker::PhantomData;

pub use admin::Admin;
pub use locking::Locking;
pub use opal_2::Opal2TPer;
pub use preconfig_shared::{INITIAL_SID_PASSWORD, PSID_PASSWORD};
pub use security_provider::{SecurityProvider, Table};

use sed_packet::discovery::{
    BlockSIDAuthDescriptor, Discovery, FeatureDescriptor, GeometryDescriptor, LockingDescriptor, OpalV2Descriptor,
    OwnerPasswordState, TPerDescriptor,
};
use sorbit::ser_de::ToBytes;

use crate::{
    device::{BASE_COM_ID, NUM_COM_IDS},
    management_session::CAPABILITIES,
};
use sed_spec::{
    methods::MethodStatus, objects::SecurityProviderRef, preconfig::opal_2::admin::c_pin, types::LifeCycleState,
};

#[derive(Debug)]
pub enum TPer {
    Opal2(Opal2TPer),
}

impl TPer {
    pub fn sp(&self, uid: SecurityProviderRef) -> Option<&dyn SecurityProvider> {
        match self {
            TPer::Opal2(tper) => tper.sp(uid),
        }
    }

    pub fn sp_mut(&mut self, uid: SecurityProviderRef) -> Option<&mut dyn SecurityProvider> {
        match self {
            TPer::Opal2(tper) => tper.sp_mut(uid),
        }
    }

    pub fn admin_sp(&self) -> &Admin {
        match self {
            TPer::Opal2(tper) => tper.admin_sp(),
        }
    }

    pub fn admin_sp_mut(&mut self) -> &mut Admin {
        match self {
            TPer::Opal2(tper) => tper.admin_sp_mut(),
        }
    }

    pub fn locking_sp(&self) -> Option<&Locking> {
        match self {
            TPer::Opal2(tper) => tper.locking_sp(),
        }
    }

    pub fn locking_sp_mut(&mut self) -> Option<&mut Locking> {
        match self {
            TPer::Opal2(tper) => tper.locking_sp_mut(),
        }
    }

    pub fn restore_preconfig(&mut self, sp: SecurityProviderRef) -> Result<(), MethodStatus> {
        match self {
            TPer::Opal2(tper) => tper.restore_preconfig(sp),
        }
    }

    pub fn discover(&self) -> Discovery {
        let fixed = [
            Self::tper_feature_desc(),
            self.ssc_feature_desc(),
            Self::geometry_feature_desc(),
            self.block_sid_authentication_desc(),
        ];
        let locking = self.locking_feature_desc();
        Discovery { feature_descriptors: fixed.into_iter().chain(locking).collect(), ..Default::default() }
    }

    pub fn pop_discovery(&self) -> Vec<u8> {
        self.discover().to_bytes().expect("serializing discovery failed")
    }

    fn tper_feature_desc() -> FeatureDescriptor {
        let desc = TPerDescriptor {
            version: PhantomData,
            length: PhantomData,
            sync_supported: true,
            async_supported: CAPABILITIES.asynchronous,
            ack_nak_supported: CAPABILITIES.ack_nak,
            buffer_mgmt_supported: CAPABILITIES.buffer_mgmt,
            streaming_supported: true,
            com_id_mgmt_supported: false,
        };
        FeatureDescriptor::TPer(desc)
    }

    fn locking_feature_desc(&self) -> Option<FeatureDescriptor> {
        self.locking_sp().map(|locking_sp| {
            let admin_sp = self.admin_sp();
            let locking_sp_obj = admin_sp.sp.get(&locking_sp.uid).expect("locking SP missing from preconfig");
            let life_cycle_state = locking_sp_obj.life_cycle_state.unwrap_or(LifeCycleState::ManufacturedInactive);
            let locking_enabled =
                life_cycle_state == LifeCycleState::Manufactured || life_cycle_state == LifeCycleState::Issued;

            let locked = locking_sp
                .locking
                .values()
                .any(|range| range.read_locked.unwrap_or(false) || range.write_locked.unwrap_or(false));

            let (_, mbr_control) =
                locking_sp.mbr_control.first_key_value().expect("MBR control missing from preconfig");
            let mbr_enabled = mbr_control.enable.unwrap_or(false);
            let mbr_done = mbr_control.done.unwrap_or(false);

            FeatureDescriptor::Locking(LockingDescriptor {
                version: PhantomData,
                length: PhantomData,
                hw_reset_supported: true,
                locked,
                locking_enabled,
                locking_supported: true,
                media_encryption: false,
                mbr_enabled,
                mbr_done,
                mbr_shadowing_not_supported: false,
            })
        })
    }

    fn ssc_feature_desc(&self) -> FeatureDescriptor {
        match self {
            TPer::Opal2(_) => FeatureDescriptor::OpalV2(OpalV2Descriptor {
                version: 1,
                length: PhantomData,
                base_com_id: BASE_COM_ID.0,
                num_com_ids: NUM_COM_IDS,
                no_range_crossing: false,
                num_locking_admins_supported: 4,
                num_locking_users_supported: 8,
                initial_owner_pw: OwnerPasswordState::SameAsMSID,
                reverted_owner_pw: OwnerPasswordState::SameAsMSID,
            }),
        }
    }

    fn geometry_feature_desc() -> FeatureDescriptor {
        let desc = GeometryDescriptor {
            version: PhantomData,
            length: PhantomData,
            align: true,
            logical_block_size: 512,
            alignment_granularity: 16,
            lowest_aligned_lba: 4,
        };
        FeatureDescriptor::Geometry(desc)
    }

    fn block_sid_authentication_desc(&self) -> FeatureDescriptor {
        let admin_sp = self.admin_sp();
        let c_pin_table = &admin_sp.c_pin;
        let c_pin_sid = c_pin_table.get(&c_pin::SID).expect("C_PIN::SID missing from preconfig");
        let c_pin_msid = c_pin_table.get(&c_pin::MSID).expect("C_PIN::MSID missing from preconfig");
        FeatureDescriptor::BlockSIDAuth(BlockSIDAuthDescriptor {
            version: 2,
            length: PhantomData,
            locking_sp_frozen: false,
            locking_sp_freeze_supported: false,
            sid_authentication_blocked: false,
            sid_msid_pin_differ: c_pin_sid.pin != c_pin_msid.pin,
            hw_reset_unblocks: false,
        })
    }
}
