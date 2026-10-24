//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::{rc::Rc, time::Duration};

use sed_packet::discovery::{
    AdditionalDataStoreTablesDescriptor, BlockSIDAuthDescriptor, DataRemovalDescriptor, Discovery,
    EnterpriseDescriptor, Feature, FeatureDescriptor, GeometryDescriptor, KeyPerIODescriptor, LockingDescriptor,
    OpalV1Descriptor, OpalV2Descriptor, OpaliteDescriptor, OwnerPasswordState, PyriteV1Descriptor, PyriteV2Descriptor,
    RubyDescriptor, TperDescriptor, UnrecognizedDescriptor,
};
use slint::VecModel;

use crate::ui_conv::FromBackend;
use sed_manager_gui_slint as ui;

fn owner_password_state_to_string(value: OwnerPasswordState) -> &'static str {
    match value {
        OwnerPasswordState::SameAsMSID => "MSID password",
        OwnerPasswordState::VendorSpecified => "Vendor-specified",
    }
}

fn yes_or_no(value: bool) -> &'static str {
    match value {
        true => "Yes",
        false => "No",
    }
}

fn duration(duration: Duration) -> String {
    if duration < Duration::from_secs(1) {
        format!("{} ms", duration.as_millis())
    } else if duration < Duration::from_secs(120) {
        format!("{} s", duration.as_millis())
    } else if duration < Duration::from_secs(7200) {
        format!("{} min", duration.as_secs().div_ceil(60))
    } else {
        format!("{} hr", duration.as_secs().div_ceil(3600))
    }
}

impl FromBackend<&TperDescriptor> for ui::DiscoveryFeature {
    fn from_backend(value: &TperDescriptor) -> Self {
        let properties = vec![
            ui::DiscoveryProperty { name: "Sync supported".into(), value: yes_or_no(value.sync_supported).into() },
            ui::DiscoveryProperty { name: "Async supported".into(), value: yes_or_no(value.async_supported).into() },
            ui::DiscoveryProperty {
                name: "ACK/NAK supported".into(),
                value: yes_or_no(value.ack_nak_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "Buffer management supported".into(),
                value: yes_or_no(value.buffer_mgmt_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "Streaming supported".into(),
                value: yes_or_no(value.streaming_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "ComID management supported".into(),
                value: yes_or_no(value.com_id_mgmt_supported).into(),
            },
        ];
        ui::DiscoveryFeature { name: value.name().into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl FromBackend<&LockingDescriptor> for ui::DiscoveryFeature {
    fn from_backend(value: &LockingDescriptor) -> Self {
        let properties = vec![
            ui::DiscoveryProperty {
                name: "Locking supported".into(),
                value: yes_or_no(value.locking_supported).into(),
            },
            ui::DiscoveryProperty { name: "Locking enabled".into(), value: yes_or_no(value.locking_enabled).into() },
            ui::DiscoveryProperty { name: "Locked".into(), value: yes_or_no(value.locked).into() },
            ui::DiscoveryProperty {
                name: "Media encryption supported".into(),
                value: yes_or_no(value.media_encryption).into(),
            },
            ui::DiscoveryProperty { name: "Shadow MBR enabled".into(), value: yes_or_no(value.mbr_enabled).into() },
            ui::DiscoveryProperty { name: "Shadow MBR done".into(), value: yes_or_no(value.mbr_done).into() },
            ui::DiscoveryProperty {
                name: "Shadow MBR supported".into(),
                value: yes_or_no(!value.mbr_shadowing_not_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "Hardware reset supported".into(),
                value: yes_or_no(value.hw_reset_supported).into(),
            },
        ];
        ui::DiscoveryFeature { name: value.name().into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl FromBackend<&GeometryDescriptor> for ui::DiscoveryFeature {
    fn from_backend(value: &GeometryDescriptor) -> Self {
        let properties = vec![
            ui::DiscoveryProperty { name: "Alignment required".into(), value: yes_or_no(value.align).into() },
            ui::DiscoveryProperty {
                name: "Logical block size".into(),
                value: value.logical_block_size.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Alignment granularity".into(),
                value: value.alignment_granularity.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Lowest aligned LBA".into(),
                value: value.lowest_aligned_lba.to_string().into(),
            },
        ];
        ui::DiscoveryFeature { name: value.name().into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl FromBackend<&DataRemovalDescriptor> for ui::DiscoveryFeature {
    fn from_backend(value: &DataRemovalDescriptor) -> Self {
        let properties = vec![
            ui::DiscoveryProperty { name: "Data removal processing".into(), value: yes_or_no(value.processing).into() },
            ui::DiscoveryProperty {
                name: "Data removal interrupted".into(),
                value: yes_or_no(value.interrupted).into(),
            },
            ui::DiscoveryProperty {
                name: "Overwrite supported".into(),
                value: yes_or_no(value.supported_mechanism.overwrite).into(),
            },
            ui::DiscoveryProperty {
                name: "Block erase supported".into(),
                value: yes_or_no(value.supported_mechanism.block_erase).into(),
            },
            ui::DiscoveryProperty {
                name: "Crypto erase supported".into(),
                value: yes_or_no(value.supported_mechanism.crypto_erase).into(),
            },
            ui::DiscoveryProperty {
                name: "Vendor erase supported".into(),
                value: yes_or_no(value.supported_mechanism.vendor_erase).into(),
            },
            ui::DiscoveryProperty {
                name: "Overwrite time".into(),
                value: value.removal_time.overwrite().map(duration).unwrap_or("-".into()).into(),
            },
            ui::DiscoveryProperty {
                name: "Block erase time".into(),
                value: value.removal_time.block_erase().map(duration).unwrap_or("-".into()).into(),
            },
            ui::DiscoveryProperty {
                name: "Crypto erase time".into(),
                value: value.removal_time.crypto_erase().map(duration).unwrap_or("-".into()).into(),
            },
            ui::DiscoveryProperty {
                name: "Vendor erase time".into(),
                value: value.removal_time.vendor_erase().map(duration).unwrap_or("-".into()).into(),
            },
        ];
        ui::DiscoveryFeature { name: value.name().into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl FromBackend<&BlockSIDAuthDescriptor> for ui::DiscoveryFeature {
    fn from_backend(value: &BlockSIDAuthDescriptor) -> Self {
        let properties = vec![
            ui::DiscoveryProperty {
                name: "SID & MSID PIN differ".into(),
                value: yes_or_no(value.sid_msid_pin_differ).into(),
            },
            ui::DiscoveryProperty {
                name: "SID authentication blocked".into(),
                value: yes_or_no(value.sid_authentication_blocked).into(),
            },
            ui::DiscoveryProperty {
                name: "Locking SP freeze supported".into(),
                value: yes_or_no(value.locking_sp_freeze_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "Locking SP frozen".into(),
                value: yes_or_no(value.locking_sp_frozen).into(),
            },
            ui::DiscoveryProperty {
                name: "Hardware reset unblocks".into(),
                value: yes_or_no(value.hw_reset_unblocks).into(),
            },
        ];
        ui::DiscoveryFeature { name: value.name().into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl FromBackend<&AdditionalDataStoreTablesDescriptor> for ui::DiscoveryFeature {
    fn from_backend(value: &AdditionalDataStoreTablesDescriptor) -> Self {
        let properties = vec![
            ui::DiscoveryProperty {
                name: "Max number of tables".into(),
                value: value.max_num_tables.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Max total size of tables".into(),
                value: value.max_total_size_of_tables.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Table size alignment".into(),
                value: value.table_size_alignment.to_string().into(),
            },
        ];
        ui::DiscoveryFeature { name: value.name().into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl FromBackend<&EnterpriseDescriptor> for ui::DiscoveryFeature {
    fn from_backend(value: &EnterpriseDescriptor) -> Self {
        let properties = vec![
            ui::DiscoveryProperty { name: "Base ComID".into(), value: value.base_com_id.to_string().into() },
            ui::DiscoveryProperty { name: "Number of ComIDs".into(), value: value.num_com_ids.to_string().into() },
            ui::DiscoveryProperty {
                name: "LBA range crossing".into(),
                value: yes_or_no(!value.no_range_crossing).into(),
            },
        ];
        ui::DiscoveryFeature { name: value.name().into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl FromBackend<&OpalV1Descriptor> for ui::DiscoveryFeature {
    fn from_backend(value: &OpalV1Descriptor) -> Self {
        let properties = vec![
            ui::DiscoveryProperty { name: "Base ComID".into(), value: value.base_com_id.to_string().into() },
            ui::DiscoveryProperty { name: "Number of ComIDs".into(), value: value.num_com_ids.to_string().into() },
            ui::DiscoveryProperty {
                name: "LBA range crossing".into(),
                value: yes_or_no(!value.no_range_crossing).into(),
            },
        ];
        ui::DiscoveryFeature { name: value.name().into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl FromBackend<&OpalV2Descriptor> for ui::DiscoveryFeature {
    fn from_backend(value: &OpalV2Descriptor) -> Self {
        let properties = vec![
            ui::DiscoveryProperty { name: "Base ComID".into(), value: value.base_com_id.to_string().into() },
            ui::DiscoveryProperty { name: "Number of ComIDs".into(), value: value.num_com_ids.to_string().into() },
            ui::DiscoveryProperty {
                name: "LBA range crossing".into(),
                value: yes_or_no(!value.no_range_crossing).into(),
            },
            ui::DiscoveryProperty {
                name: "Number of locking admins".into(),
                value: value.num_locking_admins_supported.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Number of locking users".into(),
                value: value.num_locking_users_supported.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Initial SID password".into(),
                value: owner_password_state_to_string(value.initial_owner_pw).into(),
            },
            ui::DiscoveryProperty {
                name: "Reverted SID password".into(),
                value: owner_password_state_to_string(value.reverted_owner_pw).into(),
            },
        ];
        ui::DiscoveryFeature { name: value.name().into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl FromBackend<&OpaliteDescriptor> for ui::DiscoveryFeature {
    fn from_backend(value: &OpaliteDescriptor) -> Self {
        let properties = vec![
            ui::DiscoveryProperty { name: "Base ComID".into(), value: value.base_com_id.to_string().into() },
            ui::DiscoveryProperty { name: "Number of ComIDs".into(), value: value.num_com_ids.to_string().into() },
            ui::DiscoveryProperty {
                name: "Initial SID password".into(),
                value: owner_password_state_to_string(value.initial_owner_pw).into(),
            },
            ui::DiscoveryProperty {
                name: "Reverted SID password".into(),
                value: owner_password_state_to_string(value.reverted_owner_pw).into(),
            },
        ];
        ui::DiscoveryFeature { name: value.name().into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl FromBackend<&PyriteV1Descriptor> for ui::DiscoveryFeature {
    fn from_backend(value: &PyriteV1Descriptor) -> Self {
        let properties = vec![
            ui::DiscoveryProperty { name: "Base ComID".into(), value: value.base_com_id.to_string().into() },
            ui::DiscoveryProperty { name: "Number of ComIDs".into(), value: value.num_com_ids.to_string().into() },
            ui::DiscoveryProperty {
                name: "Initial SID password".into(),
                value: owner_password_state_to_string(value.initial_owner_pw).into(),
            },
            ui::DiscoveryProperty {
                name: "Reverted SID password".into(),
                value: owner_password_state_to_string(value.reverted_owner_pw).into(),
            },
        ];
        ui::DiscoveryFeature { name: value.name().into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl FromBackend<&PyriteV2Descriptor> for ui::DiscoveryFeature {
    fn from_backend(value: &PyriteV2Descriptor) -> Self {
        let properties = vec![
            ui::DiscoveryProperty { name: "Base ComID".into(), value: value.base_com_id.to_string().into() },
            ui::DiscoveryProperty { name: "Number of ComIDs".into(), value: value.num_com_ids.to_string().into() },
            ui::DiscoveryProperty {
                name: "Initial SID password".into(),
                value: owner_password_state_to_string(value.initial_owner_pw).into(),
            },
            ui::DiscoveryProperty {
                name: "Reverted SID password".into(),
                value: owner_password_state_to_string(value.reverted_owner_pw).into(),
            },
        ];
        ui::DiscoveryFeature { name: value.name().into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl FromBackend<&RubyDescriptor> for ui::DiscoveryFeature {
    fn from_backend(value: &RubyDescriptor) -> Self {
        let properties = vec![
            ui::DiscoveryProperty { name: "Base ComID".into(), value: value.base_com_id.to_string().into() },
            ui::DiscoveryProperty { name: "Number of ComIDs".into(), value: value.num_com_ids.to_string().into() },
            ui::DiscoveryProperty {
                name: "LBA range crossing".into(),
                value: yes_or_no(!value.no_range_crossing).into(),
            },
            ui::DiscoveryProperty {
                name: "Number of locking admins".into(),
                value: value.num_locking_admins_supported.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Number of locking users".into(),
                value: value.num_locking_users_supported.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Initial SID password".into(),
                value: owner_password_state_to_string(value.initial_owner_pw).into(),
            },
            ui::DiscoveryProperty {
                name: "Reverted SID password".into(),
                value: owner_password_state_to_string(value.reverted_owner_pw).into(),
            },
        ];
        ui::DiscoveryFeature { name: value.name().into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl FromBackend<&KeyPerIODescriptor> for ui::DiscoveryFeature {
    fn from_backend(value: &KeyPerIODescriptor) -> Self {
        let properties = vec![
            ui::DiscoveryProperty {
                name: "Base ComID on protocol 1".into(),
                value: value.base_com_id_p1.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Number of ComIDs on protocol 1".into(),
                value: value.num_com_ids_p1.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Base ComID on protocol 3".into(),
                value: value.base_com_id_p3.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Number of ComIDs on protocol 3".into(),
                value: value.num_com_ids_p3.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Initial SID password".into(),
                value: owner_password_state_to_string(value.initial_owner_pw).into(),
            },
            ui::DiscoveryProperty {
                name: "Reverted SID password".into(),
                value: owner_password_state_to_string(value.reverted_owner_pw).into(),
            },
            ui::DiscoveryProperty {
                name: "Number of KPIO admins".into(),
                value: value.num_kpio_admins_supported.to_string().into(),
            },
            ui::DiscoveryProperty { name: "KPIO enabled".into(), value: yes_or_no(value.kpio_enabled).into() },
            ui::DiscoveryProperty { name: "KPIO scope".into(), value: yes_or_no(value.kpio_scope).into() },
            ui::DiscoveryProperty {
                name: "Tweak key required".into(),
                value: yes_or_no(value.tweak_key_required).into(),
            },
            ui::DiscoveryProperty {
                name: "Incorrect key detection supported".into(),
                value: yes_or_no(value.incorrect_key_detection_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "Replay protection supported".into(),
                value: yes_or_no(value.replay_protection_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "Replay protection enabled".into(),
                value: yes_or_no(value.replay_protection_enabled).into(),
            },
            ui::DiscoveryProperty {
                name: "Max key UID length".into(),
                value: value.max_key_uid_len.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "KMIP key injection supported".into(),
                value: yes_or_no(value.kmip_key_injection_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "NIST AES-KW supported".into(),
                value: yes_or_no(value.nist_aes_kw_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "NIST AES-GCM supported".into(),
                value: yes_or_no(value.nist_aes_gcm_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "NIST RSA-OAEP supported".into(),
                value: yes_or_no(value.nist_rsa_oaep_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "AES-256 wrapping supported".into(),
                value: yes_or_no(value.aes256_wrapping_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "RSA2K wrapping supported".into(),
                value: yes_or_no(value.rsa2k_wrapping_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "RSA3K wrapping supported".into(),
                value: yes_or_no(value.rsa3k_wrapping_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "RSA4K wrapping supported".into(),
                value: yes_or_no(value.rsa4k_wrapping_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "Plaintext KEK provisioning supported".into(),
                value: yes_or_no(value.plaintext_kek_prov_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "PKI KEK transport supported".into(),
                value: yes_or_no(value.pki_kek_transport_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "Number of KEKs supported".into(),
                value: value.num_keks_supported.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Total number of key tags supported".into(),
                value: value.total_key_tags_supported.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Max key tags per namespace".into(),
                value: value.max_key_tags_per_namespace.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Get nonce command nonce length".into(),
                value: value.get_nonce_cmd_nonce_len.to_string().into(),
            },
        ];
        ui::DiscoveryFeature { name: value.name().into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl FromBackend<&UnrecognizedDescriptor> for ui::DiscoveryFeature {
    fn from_backend(value: &UnrecognizedDescriptor) -> Self {
        let properties = vec![
            ui::DiscoveryProperty { name: "Version".into(), value: value.version.to_string().into() },
            ui::DiscoveryProperty { name: "Length".into(), value: value.length.to_string().into() },
        ];
        ui::DiscoveryFeature { name: "Unrecognized".into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl FromBackend<&FeatureDescriptor> for ui::DiscoveryFeature {
    fn from_backend(value: &FeatureDescriptor) -> Self {
        match value {
            FeatureDescriptor::TPer(desc) => Self::from_backend(desc),
            FeatureDescriptor::Locking(desc) => Self::from_backend(desc),
            FeatureDescriptor::Geometry(desc) => Self::from_backend(desc),
            FeatureDescriptor::DataRemoval(desc) => Self::from_backend(desc),
            FeatureDescriptor::BlockSIDAuth(desc) => Self::from_backend(desc),
            FeatureDescriptor::AdditionalDataStoreTables(desc) => Self::from_backend(desc),
            FeatureDescriptor::Enterprise(desc) => Self::from_backend(desc),
            FeatureDescriptor::OpalV1(desc) => Self::from_backend(desc),
            FeatureDescriptor::OpalV2(desc) => Self::from_backend(desc),
            FeatureDescriptor::Opalite(desc) => Self::from_backend(desc),
            FeatureDescriptor::PyriteV1(desc) => Self::from_backend(desc),
            FeatureDescriptor::PyriteV2(desc) => Self::from_backend(desc),
            FeatureDescriptor::Ruby(desc) => Self::from_backend(desc),
            FeatureDescriptor::KeyPerIO(desc) => Self::from_backend(desc),
            FeatureDescriptor::Unrecognized(code, desc) => {
                ui::DiscoveryFeature { name: format!("Unrecognized (0x{code:02x})").into(), ..Self::from_backend(desc) }
            }
        }
    }
}

impl FromBackend<&Discovery> for (ui::DeviceConfig, ui::Discovery) {
    fn from_backend(value: &Discovery) -> Self {
        let ssc_features = value
            .feature_descriptors
            .iter()
            .filter(|desc| desc.is_ssc())
            .map(ui::DiscoveryFeature::from_backend);
        let common_features = value
            .feature_descriptors
            .iter()
            .filter(|desc| !desc.is_ssc())
            .map(ui::DiscoveryFeature::from_backend);

        let mut config = ui::DeviceConfig { sid_msid_pin_differ: true, locking_enabled: false };
        if let Some(locking) = value.get::<LockingDescriptor>() {
            config.locking_enabled = locking.locking_enabled;
        }
        if let Some(block_sid_auth) = value.get::<BlockSIDAuthDescriptor>() {
            config.sid_msid_pin_differ = block_sid_auth.sid_msid_pin_differ;
        }

        let discovery = ui::Discovery {
            common_features: Rc::from(VecModel::from(common_features.collect::<Vec<_>>())).into(),
            ssc_features: Rc::from(VecModel::from(ssc_features.collect::<Vec<_>>())).into(),
            status: ui::Status { outcome: ui::Outcome::Success, ..Default::default() },
        };

        (config, discovery)
    }
}
