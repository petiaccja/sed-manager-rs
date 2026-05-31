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

use crate::display_ui::DisplayUi;
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
        format!("{} min", (duration.as_secs() + 59) / 60)
    } else {
        format!("{} hr", (duration.as_secs() + 3599) / 3600)
    }
}

impl DisplayUi for TperDescriptor {
    type Ui = ui::DiscoveryFeature;

    fn display_ui(&self) -> Self::Ui {
        let properties = vec![
            ui::DiscoveryProperty { name: "Sync supported".into(), value: yes_or_no(self.sync_supported).into() },
            ui::DiscoveryProperty { name: "Async supported".into(), value: yes_or_no(self.async_supported).into() },
            ui::DiscoveryProperty { name: "ACK/NAK supported".into(), value: yes_or_no(self.ack_nak_supported).into() },
            ui::DiscoveryProperty {
                name: "Buffer management supported".into(),
                value: yes_or_no(self.buffer_mgmt_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "Streaming supported".into(),
                value: yes_or_no(self.streaming_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "ComID management supported".into(),
                value: yes_or_no(self.com_id_mgmt_supported).into(),
            },
        ];
        ui::DiscoveryFeature { name: self.name().into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl DisplayUi for LockingDescriptor {
    type Ui = ui::DiscoveryFeature;

    fn display_ui(&self) -> Self::Ui {
        let properties = vec![
            ui::DiscoveryProperty { name: "Locking supported".into(), value: yes_or_no(self.locking_supported).into() },
            ui::DiscoveryProperty { name: "Locking enabled".into(), value: yes_or_no(self.locking_enabled).into() },
            ui::DiscoveryProperty { name: "Locked".into(), value: yes_or_no(self.locked).into() },
            ui::DiscoveryProperty {
                name: "Media encryption supported".into(),
                value: yes_or_no(self.media_encryption).into(),
            },
            ui::DiscoveryProperty { name: "Shadow MBR enabled".into(), value: yes_or_no(self.mbr_enabled).into() },
            ui::DiscoveryProperty { name: "Shadow MBR done".into(), value: yes_or_no(self.mbr_done).into() },
            ui::DiscoveryProperty {
                name: "Shadow MBR supported".into(),
                value: yes_or_no(!self.mbr_shadowing_not_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "Hardware reset supported".into(),
                value: yes_or_no(self.hw_reset_supported).into(),
            },
        ];
        ui::DiscoveryFeature { name: self.name().into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl DisplayUi for GeometryDescriptor {
    type Ui = ui::DiscoveryFeature;

    fn display_ui(&self) -> Self::Ui {
        let properties = vec![
            ui::DiscoveryProperty { name: "Alignment required".into(), value: yes_or_no(self.align).into() },
            ui::DiscoveryProperty {
                name: "Logical block size".into(),
                value: self.logical_block_size.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Alignment granularity".into(),
                value: self.alignment_granularity.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Lowest aligned LBA".into(),
                value: self.lowest_aligned_lba.to_string().into(),
            },
        ];
        ui::DiscoveryFeature { name: self.name().into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl DisplayUi for DataRemovalDescriptor {
    type Ui = ui::DiscoveryFeature;

    fn display_ui(&self) -> Self::Ui {
        let properties = vec![
            ui::DiscoveryProperty { name: "Data removal processing".into(), value: yes_or_no(self.processing).into() },
            ui::DiscoveryProperty {
                name: "Data removal interrupted".into(),
                value: yes_or_no(self.interrupted).into(),
            },
            ui::DiscoveryProperty {
                name: "Overwrite supported".into(),
                value: yes_or_no(self.supported_mechanism.overwrite).into(),
            },
            ui::DiscoveryProperty {
                name: "Block erase supported".into(),
                value: yes_or_no(self.supported_mechanism.block_erase).into(),
            },
            ui::DiscoveryProperty {
                name: "Crypto erase supported".into(),
                value: yes_or_no(self.supported_mechanism.crypto_erase).into(),
            },
            ui::DiscoveryProperty {
                name: "Vendor erase supported".into(),
                value: yes_or_no(self.supported_mechanism.vendor_erase).into(),
            },
            ui::DiscoveryProperty {
                name: "Overwrite time".into(),
                value: self.removal_time.overwrite().map(|d| duration(d)).unwrap_or("-".into()).into(),
            },
            ui::DiscoveryProperty {
                name: "Block erase time".into(),
                value: self.removal_time.block_erase().map(|d| duration(d)).unwrap_or("-".into()).into(),
            },
            ui::DiscoveryProperty {
                name: "Crypto erase time".into(),
                value: self.removal_time.crypto_erase().map(|d| duration(d)).unwrap_or("-".into()).into(),
            },
            ui::DiscoveryProperty {
                name: "Vendor erase time".into(),
                value: self.removal_time.vendor_erase().map(|d| duration(d)).unwrap_or("-".into()).into(),
            },
        ];
        ui::DiscoveryFeature { name: self.name().into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl DisplayUi for BlockSIDAuthDescriptor {
    type Ui = ui::DiscoveryFeature;

    fn display_ui(&self) -> Self::Ui {
        let properties = vec![
            ui::DiscoveryProperty {
                name: "SID & MSID PIN differ".into(),
                value: yes_or_no(self.sid_msid_pin_differ).into(),
            },
            ui::DiscoveryProperty {
                name: "SID authentication blocked".into(),
                value: yes_or_no(self.sid_authentication_blocked).into(),
            },
            ui::DiscoveryProperty {
                name: "Locking SP freeze supported".into(),
                value: yes_or_no(self.locking_sp_freeze_supported).into(),
            },
            ui::DiscoveryProperty { name: "Locking SP frozen".into(), value: yes_or_no(self.locking_sp_frozen).into() },
            ui::DiscoveryProperty {
                name: "Hardware reset unblocks".into(),
                value: yes_or_no(self.hw_reset_unblocks).into(),
            },
        ];
        ui::DiscoveryFeature { name: self.name().into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl DisplayUi for AdditionalDataStoreTablesDescriptor {
    type Ui = ui::DiscoveryFeature;

    fn display_ui(&self) -> Self::Ui {
        let properties = vec![
            ui::DiscoveryProperty {
                name: "Max number of tables".into(),
                value: self.max_num_tables.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Max total size of tables".into(),
                value: self.max_total_size_of_tables.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Table size alignment".into(),
                value: self.table_size_alignment.to_string().into(),
            },
        ];
        ui::DiscoveryFeature { name: self.name().into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl DisplayUi for EnterpriseDescriptor {
    type Ui = ui::DiscoveryFeature;

    fn display_ui(&self) -> Self::Ui {
        let properties = vec![
            ui::DiscoveryProperty { name: "Base ComID".into(), value: self.base_com_id.to_string().into() },
            ui::DiscoveryProperty { name: "Number of ComIDs".into(), value: self.num_com_ids.to_string().into() },
            ui::DiscoveryProperty {
                name: "LBA range crossing".into(),
                value: yes_or_no(!self.no_range_crossing).into(),
            },
        ];
        ui::DiscoveryFeature { name: self.name().into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl DisplayUi for OpalV1Descriptor {
    type Ui = ui::DiscoveryFeature;

    fn display_ui(&self) -> Self::Ui {
        let properties = vec![
            ui::DiscoveryProperty { name: "Base ComID".into(), value: self.base_com_id.to_string().into() },
            ui::DiscoveryProperty { name: "Number of ComIDs".into(), value: self.num_com_ids.to_string().into() },
            ui::DiscoveryProperty {
                name: "LBA range crossing".into(),
                value: yes_or_no(!self.no_range_crossing).into(),
            },
        ];
        ui::DiscoveryFeature { name: self.name().into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl DisplayUi for OpalV2Descriptor {
    type Ui = ui::DiscoveryFeature;

    fn display_ui(&self) -> Self::Ui {
        let properties = vec![
            ui::DiscoveryProperty { name: "Base ComID".into(), value: self.base_com_id.to_string().into() },
            ui::DiscoveryProperty { name: "Number of ComIDs".into(), value: self.num_com_ids.to_string().into() },
            ui::DiscoveryProperty {
                name: "LBA range crossing".into(),
                value: yes_or_no(!self.no_range_crossing).into(),
            },
            ui::DiscoveryProperty {
                name: "Number of locking admins".into(),
                value: self.num_locking_admins_supported.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Number of locking users".into(),
                value: self.num_locking_users_supported.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Initial SID password".into(),
                value: owner_password_state_to_string(self.initial_owner_pw).into(),
            },
            ui::DiscoveryProperty {
                name: "Reverted SID password".into(),
                value: owner_password_state_to_string(self.reverted_owner_pw).into(),
            },
        ];
        ui::DiscoveryFeature { name: self.name().into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl DisplayUi for OpaliteDescriptor {
    type Ui = ui::DiscoveryFeature;

    fn display_ui(&self) -> Self::Ui {
        let properties = vec![
            ui::DiscoveryProperty { name: "Base ComID".into(), value: self.base_com_id.to_string().into() },
            ui::DiscoveryProperty { name: "Number of ComIDs".into(), value: self.num_com_ids.to_string().into() },
            ui::DiscoveryProperty {
                name: "Initial SID password".into(),
                value: owner_password_state_to_string(self.initial_owner_pw).into(),
            },
            ui::DiscoveryProperty {
                name: "Reverted SID password".into(),
                value: owner_password_state_to_string(self.reverted_owner_pw).into(),
            },
        ];
        ui::DiscoveryFeature { name: self.name().into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl DisplayUi for PyriteV1Descriptor {
    type Ui = ui::DiscoveryFeature;

    fn display_ui(&self) -> Self::Ui {
        let properties = vec![
            ui::DiscoveryProperty { name: "Base ComID".into(), value: self.base_com_id.to_string().into() },
            ui::DiscoveryProperty { name: "Number of ComIDs".into(), value: self.num_com_ids.to_string().into() },
            ui::DiscoveryProperty {
                name: "Initial SID password".into(),
                value: owner_password_state_to_string(self.initial_owner_pw).into(),
            },
            ui::DiscoveryProperty {
                name: "Reverted SID password".into(),
                value: owner_password_state_to_string(self.reverted_owner_pw).into(),
            },
        ];
        ui::DiscoveryFeature { name: self.name().into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl DisplayUi for PyriteV2Descriptor {
    type Ui = ui::DiscoveryFeature;

    fn display_ui(&self) -> Self::Ui {
        let properties = vec![
            ui::DiscoveryProperty { name: "Base ComID".into(), value: self.base_com_id.to_string().into() },
            ui::DiscoveryProperty { name: "Number of ComIDs".into(), value: self.num_com_ids.to_string().into() },
            ui::DiscoveryProperty {
                name: "Initial SID password".into(),
                value: owner_password_state_to_string(self.initial_owner_pw).into(),
            },
            ui::DiscoveryProperty {
                name: "Reverted SID password".into(),
                value: owner_password_state_to_string(self.reverted_owner_pw).into(),
            },
        ];
        ui::DiscoveryFeature { name: self.name().into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl DisplayUi for RubyDescriptor {
    type Ui = ui::DiscoveryFeature;

    fn display_ui(&self) -> Self::Ui {
        let properties = vec![
            ui::DiscoveryProperty { name: "Base ComID".into(), value: self.base_com_id.to_string().into() },
            ui::DiscoveryProperty { name: "Number of ComIDs".into(), value: self.num_com_ids.to_string().into() },
            ui::DiscoveryProperty {
                name: "LBA range crossing".into(),
                value: yes_or_no(!self.no_range_crossing).into(),
            },
            ui::DiscoveryProperty {
                name: "Number of locking admins".into(),
                value: self.num_locking_admins_supported.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Number of locking users".into(),
                value: self.num_locking_users_supported.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Initial SID password".into(),
                value: owner_password_state_to_string(self.initial_owner_pw).into(),
            },
            ui::DiscoveryProperty {
                name: "Reverted SID password".into(),
                value: owner_password_state_to_string(self.reverted_owner_pw).into(),
            },
        ];
        ui::DiscoveryFeature { name: self.name().into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl DisplayUi for KeyPerIODescriptor {
    type Ui = ui::DiscoveryFeature;

    fn display_ui(&self) -> Self::Ui {
        let properties = vec![
            ui::DiscoveryProperty {
                name: "Base ComID on protocol 1".into(),
                value: self.base_com_id_p1.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Number of ComIDs on protocol 1".into(),
                value: self.num_com_ids_p1.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Base ComID on protocol 3".into(),
                value: self.base_com_id_p3.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Number of ComIDs on protocol 3".into(),
                value: self.num_com_ids_p3.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Initial SID password".into(),
                value: owner_password_state_to_string(self.initial_owner_pw).into(),
            },
            ui::DiscoveryProperty {
                name: "Reverted SID password".into(),
                value: owner_password_state_to_string(self.reverted_owner_pw).into(),
            },
            ui::DiscoveryProperty {
                name: "Number of KPIO admins".into(),
                value: self.num_kpio_admins_supported.to_string().into(),
            },
            ui::DiscoveryProperty { name: "KPIO enabled".into(), value: yes_or_no(self.kpio_enabled).into() },
            ui::DiscoveryProperty { name: "KPIO scope".into(), value: yes_or_no(self.kpio_scope).into() },
            ui::DiscoveryProperty {
                name: "Tweak key required".into(),
                value: yes_or_no(self.tweak_key_required).into(),
            },
            ui::DiscoveryProperty {
                name: "Incorrect key detection supported".into(),
                value: yes_or_no(self.incorrect_key_detection_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "Replay protection supported".into(),
                value: yes_or_no(self.replay_protection_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "Replay protection enabled".into(),
                value: yes_or_no(self.replay_protection_enabled).into(),
            },
            ui::DiscoveryProperty { name: "Max key UID length".into(), value: self.max_key_uid_len.to_string().into() },
            ui::DiscoveryProperty {
                name: "KMIP key injection supported".into(),
                value: yes_or_no(self.kmip_key_injection_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "NIST AES-KW supported".into(),
                value: yes_or_no(self.nist_aes_kw_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "NIST AES-GCM supported".into(),
                value: yes_or_no(self.nist_aes_gcm_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "NIST RSA-OAEP supported".into(),
                value: yes_or_no(self.nist_rsa_oaep_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "AES-256 wrapping supported".into(),
                value: yes_or_no(self.aes256_wrapping_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "RSA2K wrapping supported".into(),
                value: yes_or_no(self.rsa2k_wrapping_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "RSA3K wrapping supported".into(),
                value: yes_or_no(self.rsa3k_wrapping_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "RSA4K wrapping supported".into(),
                value: yes_or_no(self.rsa4k_wrapping_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "Plaintext KEK provisioning supported".into(),
                value: yes_or_no(self.plaintext_kek_prov_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "PKI KEK transport supported".into(),
                value: yes_or_no(self.pki_kek_transport_supported).into(),
            },
            ui::DiscoveryProperty {
                name: "Number of KEKs supported".into(),
                value: self.num_keks_supported.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Total number of key tags supported".into(),
                value: self.total_key_tags_supported.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Max key tags per namespace".into(),
                value: self.max_key_tags_per_namespace.to_string().into(),
            },
            ui::DiscoveryProperty {
                name: "Get nonce command nonce length".into(),
                value: self.get_nonce_cmd_nonce_len.to_string().into(),
            },
        ];
        ui::DiscoveryFeature { name: self.name().into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl DisplayUi for UnrecognizedDescriptor {
    type Ui = ui::DiscoveryFeature;

    fn display_ui(&self) -> Self::Ui {
        let properties = vec![
            ui::DiscoveryProperty { name: "Version".into(), value: self.version.to_string().into() },
            ui::DiscoveryProperty { name: "Length".into(), value: self.length.to_string().into() },
        ];
        ui::DiscoveryFeature { name: "Unrecognized".into(), properties: Rc::from(VecModel::from(properties)).into() }
    }
}

impl DisplayUi for FeatureDescriptor {
    type Ui = ui::DiscoveryFeature;

    fn display_ui(&self) -> Self::Ui {
        match self {
            FeatureDescriptor::TPer(desc) => desc.display_ui(),
            FeatureDescriptor::Locking(desc) => desc.display_ui(),
            FeatureDescriptor::Geometry(desc) => desc.display_ui(),
            FeatureDescriptor::DataRemoval(desc) => desc.display_ui(),
            FeatureDescriptor::BlockSIDAuth(desc) => desc.display_ui(),
            FeatureDescriptor::AdditionalDataStoreTables(desc) => desc.display_ui(),
            FeatureDescriptor::Enterprise(desc) => desc.display_ui(),
            FeatureDescriptor::OpalV1(desc) => desc.display_ui(),
            FeatureDescriptor::OpalV2(desc) => desc.display_ui(),
            FeatureDescriptor::Opalite(desc) => desc.display_ui(),
            FeatureDescriptor::PyriteV1(desc) => desc.display_ui(),
            FeatureDescriptor::PyriteV2(desc) => desc.display_ui(),
            FeatureDescriptor::Ruby(desc) => desc.display_ui(),
            FeatureDescriptor::KeyPerIO(desc) => desc.display_ui(),
            FeatureDescriptor::Unrecognized(code, desc) => {
                ui::DiscoveryFeature { name: format!("Unrecognized (0x{code:02x})").into(), ..desc.display_ui() }
            }
        }
    }
}

impl DisplayUi for Discovery {
    type Ui = (ui::DeviceConfig, ui::Discovery);

    fn display_ui(&self) -> Self::Ui {
        let ssc_features =
            self.feature_descriptors.iter().filter(|desc| desc.as_ssc().is_some()).map(|desc| desc.display_ui());
        let common_features =
            self.feature_descriptors.iter().filter(|desc| desc.as_ssc().is_none()).map(|desc| desc.display_ui());

        let mut config = ui::DeviceConfig { sid_msid_pin_differ: true, locking_enabled: false };
        if let Some(locking) = self.get::<LockingDescriptor>() {
            config.locking_enabled = locking.locking_enabled;
        }
        if let Some(block_sid_auth) = self.get::<BlockSIDAuthDescriptor>() {
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
