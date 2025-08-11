//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::fake_device::data::object_table::{CPINTable, LockingTable, MBRControlTable};
use crate::messaging::discovery::{
    BlockSIDAuthDescriptor, Discovery, FeatureDescriptor, GeometryDescriptor, LockingDescriptor, OpalV2Descriptor,
    OwnerPasswordState, TPerDescriptor,
};
use crate::rpc::Properties;
use crate::serialization::{OutputStream, Serialize};
use crate::spec::column_types::LifeCycleState;
use crate::spec::{self, table_id};

use super::data::SecuritySubsystemClass;

pub const BASE_COM_ID: u16 = 4100;
pub const NUM_COM_IDS: u16 = 1;

pub fn write_discovery(discovery: &Discovery, len: usize) -> Result<Vec<u8>, crate::device::Error> {
    let mut stream = OutputStream::<u8>::new();
    discovery.serialize(&mut stream).unwrap();
    let mut buffer = stream.take();
    buffer.resize(len, 0); // If the transfer length is too small, the truncated buffer must be returned.
    Ok(buffer)
}

pub fn get_discovery(properties: &Properties, ssc: &SecuritySubsystemClass) -> Discovery {
    let mut features = vec![
        get_tper_feature_desc(properties),
        get_locking_feature_desc(ssc),
        get_ssc_feature_desc(),
        get_geometry_feature_desc(),
    ];
    if let Some(block_sid_auth_desc) = get_block_sid_authentication_desc(ssc) {
        features.push(block_sid_auth_desc.into());
    }
    Discovery::new(features)
}

fn get_tper_feature_desc(properties: &Properties) -> FeatureDescriptor {
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

fn get_locking_feature_desc(ssc: &SecuritySubsystemClass) -> FeatureDescriptor {
    let locking_sp = ssc.get_sp(spec::opal::admin::sp::LOCKING).unwrap();
    let locking_enabled = ssc.get_life_cycle_state(spec::opal::admin::sp::LOCKING) == Ok(LifeCycleState::Manufactured);

    let locking_table: &LockingTable = locking_sp.get_object_table_specific(table_id::LOCKING).unwrap();
    let locked = locking_table.values().any(|range| range.read_locked || range.write_locked);

    let mbr_control_table: &MBRControlTable = locking_sp.get_object_table_specific(table_id::MBR_CONTROL).unwrap();
    let mbr_control_row = mbr_control_table.values().next().unwrap();
    let mbr_enabled = mbr_control_row.enable;
    let mbr_done = mbr_control_row.done;

    let desc = LockingDescriptor {
        hw_reset_supported: true,
        locked,
        locking_enabled,
        locking_supported: true,
        media_encryption: false,
        mbr_enabled,
        mbr_done,
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

fn get_geometry_feature_desc() -> FeatureDescriptor {
    let desc =
        GeometryDescriptor { align: true, logical_block_size: 512, alignment_granularity: 16, lowest_aligned_lba: 4 };
    FeatureDescriptor::Geometry(desc)
}

fn get_block_sid_authentication_desc(ssc: &SecuritySubsystemClass) -> Option<FeatureDescriptor> {
    let admin_sp = ssc.get_admin_sp()?;
    let c_pin_table: &CPINTable = admin_sp.get_object_table_specific(table_id::C_PIN)?;
    let c_pin_sid = c_pin_table.get(&spec::opal::admin::c_pin::SID)?;
    let c_pin_msid = c_pin_table.get(&spec::opal::admin::c_pin::MSID)?;
    Some(FeatureDescriptor::BlockSIDAuth(BlockSIDAuthDescriptor {
        locking_sp_frozen: false,
        locking_sp_freeze_supported: false,
        sid_authentication_blocked: false,
        sid_msid_pin_differ: c_pin_sid.pin != c_pin_msid.pin,
        hw_reset_unblocks: false,
    }))
}
