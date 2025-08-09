//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::fake_device::data::object_table::{LockingTable, MBRControlTable};
use crate::messaging::discovery::{
    Discovery, FeatureDescriptor, GeometryDescriptor, LockingDescriptor, OpalV2Descriptor, OwnerPasswordState,
    TPerDescriptor,
};
use crate::rpc::Properties;
use crate::serialization::{OutputStream, Serialize};
use crate::spec::column_types::LifeCycleState;
use crate::spec::{self, table_id};

use super::data::TPer;

pub const BASE_COM_ID: u16 = 4100;
pub const NUM_COM_IDS: u16 = 1;

pub fn write_discovery(discovery: &Discovery, len: usize) -> Result<Vec<u8>, crate::device::Error> {
    let mut stream = OutputStream::<u8>::new();
    discovery.serialize(&mut stream).unwrap();
    let mut buffer = stream.take();
    buffer.resize(len, 0); // If the transfer length is too small, the truncated buffer must be returned.
    Ok(buffer)
}

pub fn get_discovery(properties: &Properties, controller: &TPer) -> Discovery {
    Discovery::new(vec![
        get_tper_feature_desc(properties),
        get_locking_feature_desc(controller),
        get_ssc_feature_desc(),
        get_geometry_feature_desc(),
    ])
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

fn get_locking_feature_desc(controller: &TPer) -> FeatureDescriptor {
    let locking_sp = controller.get_security_provider(spec::opal::admin::sp::LOCKING).unwrap();
    let locking_enabled =
        controller.get_life_cycle_state(spec::opal::admin::sp::LOCKING) == Ok(LifeCycleState::Manufactured);

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
