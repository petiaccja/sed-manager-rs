//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::{rc::Rc, time::Duration};

use sed_manager::messaging::discovery::{
    AdditionalDataStoreTablesDescriptor, BlockSIDAuthDescriptor, DataRemovalDescriptor, EnterpriseDescriptor, Feature,
    FeatureDescriptor, GeometryDescriptor, KeyPerIODescriptor, LockingDescriptor, OpalV1Descriptor, OpalV2Descriptor,
    OpaliteDescriptor, OwnerPasswordState, PyriteV1Descriptor, PyriteV2Descriptor, RubyDescriptor, TPerDescriptor,
    UnrecognizedDescriptor,
};

use crate::{DeviceDiscoveryFeature, NameValuePair};

impl DeviceDiscoveryFeature {
    pub fn new(name: String, properties: Vec<NameValuePair>) -> Self {
        Self { name: name.into(), properties: Rc::from(slint::VecModel::from(properties)).into() }
    }
}

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

impl From<&TPerDescriptor> for DeviceDiscoveryFeature {
    fn from(value: &TPerDescriptor) -> Self {
        let name = format!("{} features", value.feature_code());
        let properties = vec![
            NameValuePair::new("Sync supported".into(), yes_or_no(value.sync_supported).into()),
            NameValuePair::new("Async supported".into(), yes_or_no(value.async_supported).into()),
            NameValuePair::new("ACK/NAK supported".into(), yes_or_no(value.ack_nak_supported).into()),
            NameValuePair::new("Buffer management supported".into(), yes_or_no(value.buffer_mgmt_supported).into()),
            NameValuePair::new("Streaming supported".into(), yes_or_no(value.streaming_supported).into()),
            NameValuePair::new("ComID management supported".into(), yes_or_no(value.com_id_mgmt_supported).into()),
        ];
        Self::new(name, properties)
    }
}

impl From<&LockingDescriptor> for DeviceDiscoveryFeature {
    fn from(value: &LockingDescriptor) -> Self {
        let name = format!("{} features", value.feature_code());
        let properties = vec![
            NameValuePair::new("Locking supported".into(), yes_or_no(value.locking_supported).into()),
            NameValuePair::new("Locking enabled".into(), yes_or_no(value.locking_enabled).into()),
            NameValuePair::new("Locked".into(), yes_or_no(value.locked).into()),
            NameValuePair::new("Media encryption supported".into(), yes_or_no(value.media_encryption).into()),
            NameValuePair::new("Shadow MBR enabled".into(), yes_or_no(value.mbr_enabled).into()),
            NameValuePair::new("Shadow MBR done".into(), yes_or_no(value.mbr_done).into()),
            NameValuePair::new("Shadow MBR supported".into(), yes_or_no(!value.mbr_shadowing_not_supported).into()),
            NameValuePair::new("Hardware reset supported".into(), yes_or_no(value.hw_reset_supported).into()),
        ];
        Self::new(name, properties)
    }
}

impl From<&GeometryDescriptor> for DeviceDiscoveryFeature {
    fn from(value: &GeometryDescriptor) -> Self {
        let name = format!("{} features", value.feature_code());
        let properties = vec![
            NameValuePair::new("Alignment required".into(), yes_or_no(value.align).into()),
            NameValuePair::new("Logical block size".into(), value.logical_block_size.to_string()),
            NameValuePair::new("Alignment granularity".into(), value.alignment_granularity.to_string()),
            NameValuePair::new("Lowest aligned LBA".into(), value.lowest_aligned_lba.to_string()),
        ];
        Self::new(name, properties)
    }
}

impl From<&DataRemovalDescriptor> for DeviceDiscoveryFeature {
    fn from(value: &DataRemovalDescriptor) -> Self {
        let name = format!("{} features", value.feature_code());
        let properties = vec![
            NameValuePair::new("Data removal processing".into(), yes_or_no(value.processing).into()),
            NameValuePair::new("Data removal interrupted".into(), yes_or_no(value.interrupted).into()),
            NameValuePair::new("Overwrite supported".into(), yes_or_no(value.supported_mechanism.overwrite).into()),
            NameValuePair::new("Block erase supported".into(), yes_or_no(value.supported_mechanism.block_erase).into()),
            NameValuePair::new(
                "Crypto erase supported".into(),
                yes_or_no(value.supported_mechanism.crypto_erase).into(),
            ),
            NameValuePair::new(
                "Vendor erase supported".into(),
                yes_or_no(value.supported_mechanism.vendor_erase).into(),
            ),
            NameValuePair::new(
                "Overwrite time".into(),
                value.removal_time.overwrite().map(|d| duration(d)).unwrap_or("-".into()),
            ),
            NameValuePair::new(
                "Block erase time".into(),
                value.removal_time.block_erase().map(|d| duration(d)).unwrap_or("-".into()),
            ),
            NameValuePair::new(
                "Crypto erase time".into(),
                value.removal_time.crypto_erase().map(|d| duration(d)).unwrap_or("-".into()),
            ),
            NameValuePair::new(
                "Vendor erase time".into(),
                value.removal_time.vendor_erase().map(|d| duration(d)).unwrap_or("-".into()),
            ),
        ];
        Self::new(name, properties)
    }
}

impl From<&BlockSIDAuthDescriptor> for DeviceDiscoveryFeature {
    fn from(value: &BlockSIDAuthDescriptor) -> Self {
        let name = format!("{} features", value.feature_code());
        let properties = vec![
            NameValuePair::new("SID's PIN same as MSID".into(), yes_or_no(value.sid_pin_same_as_msid).into()),
            NameValuePair::new(
                "SID's authentication blocked".into(),
                yes_or_no(value.sid_authentication_blocked).into(),
            ),
            NameValuePair::new(
                "Locking SP freeze supported".into(),
                yes_or_no(value.locking_sp_freeze_supported).into(),
            ),
            NameValuePair::new("Locking SP frozen".into(), yes_or_no(value.locking_sp_frozen).into()),
        ];
        Self::new(name, properties)
    }
}

impl From<&AdditionalDataStoreTablesDescriptor> for DeviceDiscoveryFeature {
    fn from(value: &AdditionalDataStoreTablesDescriptor) -> Self {
        let name = format!("{} features", value.feature_code());

        let properties = vec![
            NameValuePair::new("Max number of tables".into(), value.max_num_tables.to_string()),
            NameValuePair::new("Max total size of tables".into(), value.max_total_size_of_tables.to_string()),
            NameValuePair::new("Table size alignment".into(), value.table_size_alignment.to_string()),
        ];
        Self::new(name, properties)
    }
}

impl From<&EnterpriseDescriptor> for DeviceDiscoveryFeature {
    fn from(value: &EnterpriseDescriptor) -> Self {
        let name = format!("{} features", value.feature_code());
        let properties = vec![
            NameValuePair::new("Base ComID".into(), value.base_com_id.to_string()),
            NameValuePair::new("Number of ComIDs".into(), value.num_com_ids.to_string()),
            NameValuePair::new("LBA range crossing".into(), yes_or_no(!value.no_range_crossing).into()),
        ];
        Self::new(name, properties)
    }
}

impl From<&OpalV1Descriptor> for DeviceDiscoveryFeature {
    fn from(value: &OpalV1Descriptor) -> Self {
        let name = format!("{} features", value.feature_code());
        let properties = vec![
            NameValuePair::new("Base ComID".into(), value.base_com_id.to_string()),
            NameValuePair::new("Number of ComIDs".into(), value.num_com_ids.to_string()),
            NameValuePair::new("LBA range crossing".into(), yes_or_no(!value.no_range_crossing).into()),
        ];
        Self::new(name, properties)
    }
}

impl From<&OpalV2Descriptor> for DeviceDiscoveryFeature {
    fn from(value: &OpalV2Descriptor) -> Self {
        let name = format!("{} features", value.feature_code());

        let properties = vec![
            NameValuePair::new("Base ComID".into(), value.base_com_id.to_string()),
            NameValuePair::new("Number of ComIDs".into(), value.num_com_ids.to_string()),
            NameValuePair::new("LBA range crossing".into(), yes_or_no(!value.no_range_crossing).into()),
            NameValuePair::new("Number of locking admins".into(), value.num_locking_admins_supported.to_string()),
            NameValuePair::new("Number of locking users".into(), value.num_locking_users_supported.to_string()),
            NameValuePair::new(
                "Initial SID password".into(),
                owner_password_state_to_string(value.initial_owner_pw).into(),
            ),
            NameValuePair::new(
                "Reverted SID password".into(),
                owner_password_state_to_string(value.reverted_owner_pw).into(),
            ),
        ];
        Self::new(name, properties)
    }
}

impl From<&OpaliteDescriptor> for DeviceDiscoveryFeature {
    fn from(value: &OpaliteDescriptor) -> Self {
        let name = format!("{} features", value.feature_code());

        let properties = vec![
            NameValuePair::new("Base ComID".into(), value.base_com_id.to_string()),
            NameValuePair::new("Number of ComIDs".into(), value.num_com_ids.to_string()),
            NameValuePair::new(
                "Initial SID password".into(),
                owner_password_state_to_string(value.initial_owner_pw).into(),
            ),
            NameValuePair::new(
                "Reverted SID password".into(),
                owner_password_state_to_string(value.reverted_owner_pw).into(),
            ),
        ];
        Self::new(name, properties)
    }
}

impl From<&PyriteV1Descriptor> for DeviceDiscoveryFeature {
    fn from(value: &PyriteV1Descriptor) -> Self {
        let name = format!("{} features", value.feature_code());

        let properties = vec![
            NameValuePair::new("Base ComID".into(), value.base_com_id.to_string()),
            NameValuePair::new("Number of ComIDs".into(), value.num_com_ids.to_string()),
            NameValuePair::new(
                "Initial SID password".into(),
                owner_password_state_to_string(value.initial_owner_pw).into(),
            ),
            NameValuePair::new(
                "Reverted SID password".into(),
                owner_password_state_to_string(value.reverted_owner_pw).into(),
            ),
        ];
        Self::new(name, properties)
    }
}

impl From<&PyriteV2Descriptor> for DeviceDiscoveryFeature {
    fn from(value: &PyriteV2Descriptor) -> Self {
        let name = format!("{} features", value.feature_code());

        let properties = vec![
            NameValuePair::new("Base ComID".into(), value.base_com_id.to_string()),
            NameValuePair::new("Number of ComIDs".into(), value.num_com_ids.to_string()),
            NameValuePair::new(
                "Initial SID password".into(),
                owner_password_state_to_string(value.initial_owner_pw).into(),
            ),
            NameValuePair::new(
                "Reverted SID password".into(),
                owner_password_state_to_string(value.reverted_owner_pw).into(),
            ),
        ];
        Self::new(name, properties)
    }
}

impl From<&RubyDescriptor> for DeviceDiscoveryFeature {
    fn from(value: &RubyDescriptor) -> Self {
        let name = format!("{} features", value.feature_code());

        let properties = vec![
            NameValuePair::new("Base ComID".into(), value.base_com_id.to_string()),
            NameValuePair::new("Number of ComIDs".into(), value.num_com_ids.to_string()),
            NameValuePair::new("LBA range crossing".into(), yes_or_no(!value.no_range_crossing).into()),
            NameValuePair::new("Number of locking admins".into(), value.num_locking_admins_supported.to_string()),
            NameValuePair::new("Number of locking users".into(), value.num_locking_users_supported.to_string()),
            NameValuePair::new(
                "Initial SID password".into(),
                owner_password_state_to_string(value.initial_owner_pw).into(),
            ),
            NameValuePair::new(
                "Reverted SID password".into(),
                owner_password_state_to_string(value.reverted_owner_pw).into(),
            ),
        ];
        Self::new(name, properties)
    }
}

impl From<&KeyPerIODescriptor> for DeviceDiscoveryFeature {
    fn from(value: &KeyPerIODescriptor) -> Self {
        let name = format!("{} features", value.feature_code());

        let properties = vec![
            NameValuePair::new("Base ComID on protocol 1".into(), value.base_com_id_p1.to_string()),
            NameValuePair::new("Number of ComIDs on protocol 1".into(), value.num_com_ids_p1.to_string()),
            NameValuePair::new("Base ComID on protocol 3".into(), value.base_com_id_p3.to_string()),
            NameValuePair::new("Number of ComIDs on protocol 3".into(), value.num_com_ids_p3.to_string()),
            NameValuePair::new(
                "Initial SID password".into(),
                owner_password_state_to_string(value.initial_owner_pw).into(),
            ),
            NameValuePair::new(
                "Reverted SID password".into(),
                owner_password_state_to_string(value.reverted_owner_pw).into(),
            ),
            NameValuePair::new("Number of KPIO admins".into(), value.num_kpio_admins_supported.to_string()),
            NameValuePair::new("KPIO enabled".into(), yes_or_no(value.kpio_enabled).into()),
            NameValuePair::new("KPIO scope".into(), yes_or_no(value.kpio_scope).into()),
            NameValuePair::new("Tweak key required".into(), yes_or_no(value.tweak_key_required).into()),
            NameValuePair::new(
                "Incorrect key detection supported".into(),
                yes_or_no(value.incorrect_key_detection_supported).into(),
            ),
            NameValuePair::new(
                "Replay protection supported".into(),
                yes_or_no(value.replay_protection_supported).into(),
            ),
            NameValuePair::new("Replay protection enabled".into(), yes_or_no(value.replay_protection_enabled).into()),
            NameValuePair::new("Max key UID length".into(), value.max_key_uid_len.to_string()),
            NameValuePair::new(
                "KMIP key injection supported".into(),
                yes_or_no(value.kmip_key_injection_supported).into(),
            ),
            NameValuePair::new("NIST AES-KW supported".into(), yes_or_no(value.nist_aes_kw_supported).into()),
            NameValuePair::new("NIST AES-GCM supported".into(), yes_or_no(value.nist_aes_gcm_supported).into()),
            NameValuePair::new("NIST RSA-OAEP supported".into(), yes_or_no(value.nist_rsa_oaep_supported).into()),
            NameValuePair::new("AES-256 wrapping supported".into(), yes_or_no(value.aes256_wrapping_supported).into()),
            NameValuePair::new("RSA2K wrapping supported".into(), yes_or_no(value.rsa2k_wrapping_supported).into()),
            NameValuePair::new("RSA3K wrapping supported".into(), yes_or_no(value.rsa3k_wrapping_supported).into()),
            NameValuePair::new("RSA4K wrapping supported".into(), yes_or_no(value.rsa4k_wrapping_supported).into()),
            NameValuePair::new(
                "Plaintext KEK provisioning supported".into(),
                yes_or_no(value.plaintext_kek_prov_supported).into(),
            ),
            NameValuePair::new(
                "PKI KEK transport supported".into(),
                yes_or_no(value.pki_kek_transport_supported).into(),
            ),
            NameValuePair::new("Number of KEKs supported".into(), value.num_keks_supported.to_string()),
            NameValuePair::new("Total number of key tags supported".into(), value.total_key_tags_supported.to_string()),
            NameValuePair::new("Max key tags per namespace".into(), value.max_key_tags_per_namespace.to_string()),
            NameValuePair::new("Get nonce command nonce length".into(), value.get_nonce_cmd_nonce_len.to_string()),
        ];
        Self::new(name, properties)
    }
}

impl From<&UnrecognizedDescriptor> for DeviceDiscoveryFeature {
    fn from(value: &UnrecognizedDescriptor) -> Self {
        let name = format!("Unrecognized features");

        let properties = vec![
            NameValuePair::new("Feature code".into(), format!("{:#6x}", value.feature_code)),
            NameValuePair::new("Version".into(), value.version.to_string()),
            NameValuePair::new("Length".into(), value.length.to_string()),
        ];
        Self::new(name, properties)
    }
}

impl From<&FeatureDescriptor> for DeviceDiscoveryFeature {
    fn from(value: &FeatureDescriptor) -> Self {
        match value {
            FeatureDescriptor::TPer(desc) => desc.into(),
            FeatureDescriptor::Locking(desc) => desc.into(),
            FeatureDescriptor::Geometry(desc) => desc.into(),
            FeatureDescriptor::DataRemoval(desc) => desc.into(),
            FeatureDescriptor::BlockSIDAuth(desc) => desc.into(),
            FeatureDescriptor::AdditionalDataStoreTables(desc) => desc.into(),
            FeatureDescriptor::Enterprise(desc) => desc.into(),
            FeatureDescriptor::OpalV1(desc) => desc.into(),
            FeatureDescriptor::OpalV2(desc) => desc.into(),
            FeatureDescriptor::Opalite(desc) => desc.into(),
            FeatureDescriptor::PyriteV1(desc) => desc.into(),
            FeatureDescriptor::PyriteV2(desc) => desc.into(),
            FeatureDescriptor::Ruby(desc) => desc.into(),
            FeatureDescriptor::KeyPerIO(desc) => desc.into(),
            FeatureDescriptor::Unrecognized(desc) => desc.into(),
        }
    }
}
