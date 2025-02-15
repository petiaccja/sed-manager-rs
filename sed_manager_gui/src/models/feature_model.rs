use std::{rc::Rc, time::Duration};

use sed_manager::messaging::discovery::{
    DataRemovalDescriptor, EnterpriseDescriptor, Feature, FeatureDescriptor, GeometryDescriptor, KeyPerIODescriptor,
    LockingDescriptor, OpalV1Descriptor, OpalV2Descriptor, OpaliteDescriptor, OwnerPasswordState, PyriteV1Descriptor,
    PyriteV2Descriptor, RubyDescriptor, TPerDescriptor, UnrecognizedDescriptor,
};

use crate::generated::FeatureModel;

impl FeatureModel {
    pub fn new(name: String, nvps: Vec<(String, String)>) -> Self {
        let nvps: Vec<(slint::SharedString, slint::SharedString)> =
            nvps.into_iter().map(|(name, value)| (name.into(), value.into())).collect();
        Self { name: name.into(), nvps: Rc::from(slint::VecModel::from(nvps)).into() }
    }
}

fn owner_password_state_to_string(value: OwnerPasswordState) -> &'static str {
    match value {
        OwnerPasswordState::SameAsMSID => "Same as MSID",
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
        format!("{} second(s)", duration.as_millis())
    } else if duration < Duration::from_secs(7200) {
        format!("{} minute(s)", (duration.as_secs() + 59) / 60)
    } else {
        format!("{} hour(s)", (duration.as_secs() + 3599) / 3600)
    }
}

impl From<&TPerDescriptor> for FeatureModel {
    fn from(value: &TPerDescriptor) -> Self {
        let name = format!("{} features", value.feature_code());
        let nvps = vec![
            ("Sync supported".into(), yes_or_no(value.sync_supported).into()),
            ("Async supported".into(), yes_or_no(value.async_supported).into()),
            ("ACK/NAK supported".into(), yes_or_no(value.ack_nak_supported).into()),
            ("Buffer management supported".into(), yes_or_no(value.buffer_mgmt_supported).into()),
            ("Streaming supported".into(), yes_or_no(value.streaming_supported).into()),
            ("Com ID management supported".into(), yes_or_no(value.com_id_mgmt_supported).into()),
        ];
        Self::new(name, nvps)
    }
}

impl From<&LockingDescriptor> for FeatureModel {
    fn from(value: &LockingDescriptor) -> Self {
        let name = format!("{} features", value.feature_code());

        let nvps = vec![
            ("Locking supported".into(), yes_or_no(value.locking_supported).into()),
            ("Locking enabled".into(), yes_or_no(value.locking_enabled).into()),
            ("Locked".into(), yes_or_no(value.locked).into()),
            ("Media encryption supported".into(), yes_or_no(value.media_encryption).into()),
            ("Shadow MBR enabled".into(), yes_or_no(value.mbr_enabled).into()),
            ("Shadow MBR done".into(), yes_or_no(value.mbr_done).into()),
            ("Shadow MBR supported".into(), yes_or_no(!value.mbr_shadowing_not_supported).into()),
            ("HW reset supported".into(), yes_or_no(value.hw_reset_supported).into()),
        ];
        Self::new(name, nvps)
    }
}

impl From<&GeometryDescriptor> for FeatureModel {
    fn from(value: &GeometryDescriptor) -> Self {
        let name = format!("{} features", value.feature_code());

        let nvps = vec![
            ("Alignment required".into(), yes_or_no(value.align).into()),
            ("Logical block size".into(), value.logical_block_size.to_string()),
            ("Alignment granularity".into(), value.alignment_granularity.to_string()),
            ("Lowest aligned LBA".into(), value.lowest_aligned_lba.to_string()),
        ];
        Self::new(name, nvps)
    }
}

impl From<&DataRemovalDescriptor> for FeatureModel {
    fn from(value: &DataRemovalDescriptor) -> Self {
        let name = format!("{} features", value.feature_code());

        let nvps = vec![
            ("Data removal processing".into(), yes_or_no(value.processing).into()),
            ("Data removal interrupted".into(), yes_or_no(value.interrupted).into()),
            ("Overwrite supported".into(), yes_or_no(value.supported_mechanism.overwrite).into()),
            ("Block erase supported".into(), yes_or_no(value.supported_mechanism.block_erase).into()),
            ("Crypto erase supported".into(), yes_or_no(value.supported_mechanism.crypto_erase).into()),
            ("Vendor erase supported".into(), yes_or_no(value.supported_mechanism.vendor_erase).into()),
            ("Overwrite time".into(), value.removal_time.overwrite().map(|d| duration(d)).unwrap_or("-".into())),
            (
                "Block erase time".into(),
                value.removal_time.block_erase().map(|d| duration(d)).unwrap_or("-".into()),
            ),
            (
                "Crypto erase time".into(),
                value.removal_time.crypto_erase().map(|d| duration(d)).unwrap_or("-".into()),
            ),
            (
                "Vendor erase time".into(),
                value.removal_time.vendor_erase().map(|d| duration(d)).unwrap_or("-".into()),
            ),
        ];
        Self::new(name, nvps)
    }
}

impl From<&EnterpriseDescriptor> for FeatureModel {
    fn from(value: &EnterpriseDescriptor) -> Self {
        let name = format!("{} features", value.feature_code());

        let nvps = vec![
            ("Base com ID".into(), value.base_com_id.to_string()),
            ("Nr. of com IDs".into(), value.num_com_ids.to_string()),
            ("LBA range crossing".into(), yes_or_no(!value.no_range_crossing).into()),
        ];
        Self::new(name, nvps)
    }
}

impl From<&OpalV1Descriptor> for FeatureModel {
    fn from(value: &OpalV1Descriptor) -> Self {
        let name = format!("{} features", value.feature_code());

        let nvps = vec![
            ("Base com ID".into(), value.base_com_id.to_string()),
            ("Nr. of com IDs".into(), value.num_com_ids.to_string()),
            ("LBA range crossing".into(), yes_or_no(!value.no_range_crossing).into()),
        ];
        Self::new(name, nvps)
    }
}

impl From<&OpalV2Descriptor> for FeatureModel {
    fn from(value: &OpalV2Descriptor) -> Self {
        let name = format!("{} features", value.feature_code());

        let nvps = vec![
            ("Base com ID".into(), value.base_com_id.to_string()),
            ("Nr. of com IDs".into(), value.num_com_ids.to_string()),
            ("LBA range crossing".into(), yes_or_no(!value.no_range_crossing).into()),
            ("Nr. of Locking SP admins".into(), value.num_locking_admins_supported.to_string()),
            ("Nr. or Locking SP users".into(), value.num_locking_users_supported.to_string()),
            ("Initial SID password".into(), owner_password_state_to_string(value.initial_owner_pw).into()),
            ("Reverted SID password".into(), owner_password_state_to_string(value.reverted_owner_pw).into()),
        ];
        Self::new(name, nvps)
    }
}

impl From<&OpaliteDescriptor> for FeatureModel {
    fn from(value: &OpaliteDescriptor) -> Self {
        let name = format!("{} features", value.feature_code());

        let nvps = vec![
            ("Base com ID".into(), value.base_com_id.to_string()),
            ("Nr. of com IDs".into(), value.num_com_ids.to_string()),
            ("Initial SID password".into(), owner_password_state_to_string(value.initial_owner_pw).into()),
            ("Reverted SID password".into(), owner_password_state_to_string(value.reverted_owner_pw).into()),
        ];
        Self::new(name, nvps)
    }
}

impl From<&PyriteV1Descriptor> for FeatureModel {
    fn from(value: &PyriteV1Descriptor) -> Self {
        let name = format!("{} features", value.feature_code());

        let nvps = vec![
            ("Base com ID".into(), value.base_com_id.to_string()),
            ("Nr. of com IDs".into(), value.num_com_ids.to_string()),
            ("Initial SID password".into(), owner_password_state_to_string(value.initial_owner_pw).into()),
            ("Reverted SID password".into(), owner_password_state_to_string(value.reverted_owner_pw).into()),
        ];
        Self::new(name, nvps)
    }
}

impl From<&PyriteV2Descriptor> for FeatureModel {
    fn from(value: &PyriteV2Descriptor) -> Self {
        let name = format!("{} features", value.feature_code());

        let nvps = vec![
            ("Base com ID".into(), value.base_com_id.to_string()),
            ("Nr. of com IDs".into(), value.num_com_ids.to_string()),
            ("Initial SID password".into(), owner_password_state_to_string(value.initial_owner_pw).into()),
            ("Reverted SID password".into(), owner_password_state_to_string(value.reverted_owner_pw).into()),
        ];
        Self::new(name, nvps)
    }
}

impl From<&RubyDescriptor> for FeatureModel {
    fn from(value: &RubyDescriptor) -> Self {
        let name = format!("{} features", value.feature_code());

        let nvps = vec![
            ("Base com ID".into(), value.base_com_id.to_string()),
            ("Nr. of com IDs".into(), value.num_com_ids.to_string()),
            ("LBA range crossing".into(), yes_or_no(!value.no_range_crossing).into()),
            ("Nr. of Locking SP admins".into(), value.num_locking_admins_supported.to_string()),
            ("Nr. or Locking SP users".into(), value.num_locking_users_supported.to_string()),
            ("Initial SID password".into(), owner_password_state_to_string(value.initial_owner_pw).into()),
            ("Reverted SID password".into(), owner_password_state_to_string(value.reverted_owner_pw).into()),
        ];
        Self::new(name, nvps)
    }
}

impl From<&KeyPerIODescriptor> for FeatureModel {
    fn from(value: &KeyPerIODescriptor) -> Self {
        let name = format!("{} features", value.feature_code());

        let nvps = vec![
            ("Base com ID 0x01".into(), value.base_com_id_p1.to_string()),
            ("Nr. of com IDs 0x01".into(), value.num_com_ids_p1.to_string()),
            ("Base com ID 0x03".into(), value.base_com_id_p3.to_string()),
            ("Nr. of com IDs 0x03".into(), value.num_com_ids_p3.to_string()),
            ("Initial SID password".into(), owner_password_state_to_string(value.initial_owner_pw).into()),
            ("Reverted SID password".into(), owner_password_state_to_string(value.reverted_owner_pw).into()),
            ("Nr. of KPIO admins".into(), value.num_kpio_admins_supported.to_string()),
            ("KPIO enabled".into(), yes_or_no(value.kpio_enabled).into()),
            ("KPIO scope".into(), yes_or_no(value.kpio_scope).into()),
            ("Tweak key required".into(), yes_or_no(value.tweak_key_required).into()),
            (
                "Incorrect key detection supported".into(),
                yes_or_no(value.incorrect_key_detection_supported).into(),
            ),
            ("Replay protection supported".into(), yes_or_no(value.replay_protection_supported).into()),
            ("Replay protection enabled".into(), yes_or_no(value.replay_protection_enabled).into()),
            ("Max. key UID length".into(), value.max_key_uid_len.to_string()),
            ("KMIP key injection supported".into(), yes_or_no(value.kmip_key_injection_supported).into()),
            ("NIST AES-KW supported".into(), yes_or_no(value.nist_aes_kw_supported).into()),
            ("NIST AES-GCM supported".into(), yes_or_no(value.nist_aes_gcm_supported).into()),
            ("NIST RSA-OAEP supported".into(), yes_or_no(value.nist_rsa_oaep_supported).into()),
            ("AES-256 wrapping supported".into(), yes_or_no(value.aes256_wrapping_supported).into()),
            ("RSA2K wrapping supported".into(), yes_or_no(value.rsa2k_wrapping_supported).into()),
            ("RSA3K wrapping supported".into(), yes_or_no(value.rsa3k_wrapping_supported).into()),
            ("RSA4K wrapping supported".into(), yes_or_no(value.rsa4k_wrapping_supported).into()),
            ("Plaintext KEK provisioning supported".into(), yes_or_no(value.plaintext_kek_prov_supported).into()),
            ("PKI KEK transport supported".into(), yes_or_no(value.pki_kek_transport_supported).into()),
            ("Nr. of KEKs supported".into(), value.num_keks_supported.to_string()),
            ("Total nr. of key tags supported".into(), value.total_key_tags_supported.to_string()),
            ("Max. key tags per namespace".into(), value.max_key_tags_per_namespace.to_string()),
            ("Get none command none length".into(), value.get_nonce_cmd_nonce_len.to_string()),
        ];
        Self::new(name, nvps)
    }
}

impl From<&UnrecognizedDescriptor> for FeatureModel {
    fn from(value: &UnrecognizedDescriptor) -> Self {
        let name = format!("Unrecognized features");

        let nvps = vec![
            ("Feature code".into(), format!("{:#6x}", value.feature_code)),
            ("Version".into(), value.version.to_string()),
            ("Length".into(), value.length.to_string()),
        ];
        Self::new(name, nvps)
    }
}

impl From<&FeatureDescriptor> for FeatureModel {
    fn from(value: &FeatureDescriptor) -> Self {
        match value {
            FeatureDescriptor::TPer(desc) => desc.into(),
            FeatureDescriptor::Locking(desc) => desc.into(),
            FeatureDescriptor::Geometry(desc) => desc.into(),
            FeatureDescriptor::DataRemoval(desc) => desc.into(),
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
