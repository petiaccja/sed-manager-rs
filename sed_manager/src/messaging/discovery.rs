use core::ops::{Deref, DerefMut};
use core::time::Duration;

use crate::serialization::{
    vec_with_len::VecWithLen, Deserialize, Error as SerializeError, InputStream, ItemRead, OutputStream, Serialize,
};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum FeatureCode {
    TPer = 0x0001,
    Locking = 0x0002,
    Geometry = 0x0003,
    DataRemoval = 0x0404,
    BlockSIDAuth = 0x0402,
    AdditionalDataStoreTables = 0x0202,
    Enterprise = 0x0100,
    OpalV1 = 0x0200,
    OpalV2 = 0x0203,
    Opalite = 0x0301,
    PyriteV1 = 0x0302,
    PyriteV2 = 0x0303,
    Ruby = 0x0304,
    KeyPerIO = 0x0305,
    #[fallback]
    Unrecognized = 0xFFFF,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum OwnerPasswordState {
    SameAsMSID = 0x00,
    VendorSpecified = 0xFF,
}

pub trait Feature {
    fn feature_code(&self) -> FeatureCode;
    fn version(&self) -> u8;
}

pub trait SecuritySubsystemClass: Feature {
    fn base_com_id(&self) -> u16;
    fn num_com_ids(&self) -> u16;
    fn base_com_id_p3(&self) -> Option<u16> {
        None
    }
    fn num_com_ids_p3(&self) -> Option<u16> {
        None
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[layout(round = 12)]
pub struct TPerDescriptor {
    #[layout(offset = 0, bits = 1..=1)]
    pub com_id_mgmt_supported: bool,
    #[layout(offset = 0, bits = 3..=3)]
    pub streaming_supported: bool,
    #[layout(offset = 0, bits = 4..=4)]
    pub buffer_mgmt_supported: bool,
    #[layout(offset = 0, bits = 5..=5)]
    pub ack_nak_supported: bool,
    #[layout(offset = 0, bits = 6..=6)]
    pub async_supported: bool,
    #[layout(offset = 0, bits = 7..=7)]
    pub sync_supported: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[layout(round = 12)]
pub struct LockingDescriptor {
    #[layout(offset = 0, bits = 0..=0)]
    pub hw_reset_supported: bool,
    #[layout(offset = 0, bits = 1..=1)]
    pub mbr_shadowing_not_supported: bool,
    #[layout(offset = 0, bits = 2..=2)]
    pub mbr_done: bool,
    #[layout(offset = 0, bits = 3..=3)]
    pub mbr_enabled: bool,
    #[layout(offset = 0, bits = 4..=4)]
    pub media_encryption: bool,
    #[layout(offset = 0, bits = 5..=5)]
    pub locked: bool,
    #[layout(offset = 0, bits = 6..=6)]
    pub locking_enabled: bool,
    #[layout(offset = 0, bits = 7..=7)]
    pub locking_supported: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct GeometryDescriptor {
    #[layout(offset = 0, bits = 7..=7)]
    pub align: bool,
    #[layout(offset = 8)]
    pub logical_block_size: u32,
    pub alignment_granularity: u64,
    pub lowest_aligned_lba: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DataRemovalMechanism {
    #[layout(offset = 2, bits = 2..=2)]
    pub vendor_erase: bool,
    #[layout(offset = 2, bits = 5..=5)]
    pub crypto_erase: bool,
    #[layout(offset = 2, bits = 6..=6)]
    pub block_erase: bool,
    #[layout(offset = 2, bits = 7..=7)]
    pub overwrite: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DataRemovalTime {
    #[layout(offset = 0, bits = 2..=2)]
    pub vendor_erase_unit: bool,
    #[layout(offset = 0, bits = 5..=5)]
    pub crypto_erase_unit: bool,
    #[layout(offset = 0, bits = 6..=6)]
    pub block_erase_unit: bool,
    #[layout(offset = 0, bits = 7..=7)]
    pub overwrite_unit: bool,
    #[layout(offset = 1)]
    pub overwrite_amount: u16,
    #[layout(offset = 3)]
    pub block_erase_amount: u16,
    #[layout(offset = 5)]
    pub crypto_erase_amount: u16,
    #[layout(offset = 11)]
    pub vendor_erase_amount: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[layout(round = 32)]
pub struct DataRemovalDescriptor {
    #[layout(offset = 1, bits = 6..=6)]
    pub interrupted: bool,
    #[layout(offset = 1, bits = 7..=7)]
    pub processing: bool,
    #[layout(offset = 2)]
    pub supported_mechanism: DataRemovalMechanism,
    pub removal_time: DataRemovalTime,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[layout(round = 12)]
pub struct BlockSIDAuthDescriptor {
    #[layout(offset = 0, bits = 4..=4)]
    pub locking_sp_frozen: bool,
    #[layout(offset = 0, bits = 5..=5)]
    pub locking_sp_freeze_supported: bool,
    #[layout(offset = 0, bits = 6..=6)]
    pub sid_authentication_blocked: bool,
    #[layout(offset = 0, bits = 7..=7)]
    pub sid_pin_same_as_msid: bool,
    #[layout(offset = 1, bits = 7..=7)]
    pub hw_reset_unblocks: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[layout(round = 12)]
pub struct AdditionalDataStoreTablesDescriptor {
    #[layout(offset = 2)]
    pub max_num_tables: u16,
    pub max_total_size_of_tables: u32,
    pub table_size_alignment: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[layout(round = 16)]
pub struct EnterpriseDescriptor {
    pub base_com_id: u16,
    pub num_com_ids: u16,
    #[layout(offset = 4, bits = 7..=7)]
    pub no_range_crossing: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[layout(round = 16)]
pub struct OpalV1Descriptor {
    pub base_com_id: u16,
    pub num_com_ids: u16,
    #[layout(offset = 4, bits = 7..=7)]
    pub no_range_crossing: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[layout(round = 16)]
pub struct OpalV2Descriptor {
    pub base_com_id: u16,
    pub num_com_ids: u16,
    #[layout(offset = 4, bits = 7..=7)]
    pub no_range_crossing: bool,
    #[layout(offset = 5)]
    pub num_locking_admins_supported: u16,
    pub num_locking_users_supported: u16,
    pub initial_owner_pw: OwnerPasswordState,
    pub reverted_owner_pw: OwnerPasswordState,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[layout(round = 16)]
pub struct OpaliteDescriptor {
    pub base_com_id: u16,
    pub num_com_ids: u16,
    #[layout(offset = 9)]
    pub initial_owner_pw: OwnerPasswordState,
    pub reverted_owner_pw: OwnerPasswordState,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[layout(round = 16)]
pub struct PyriteV1Descriptor {
    pub base_com_id: u16,
    pub num_com_ids: u16,
    #[layout(offset = 9)]
    pub initial_owner_pw: OwnerPasswordState,
    pub reverted_owner_pw: OwnerPasswordState,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[layout(round = 16)]
pub struct PyriteV2Descriptor {
    pub base_com_id: u16,
    pub num_com_ids: u16,
    #[layout(offset = 9)]
    pub initial_owner_pw: OwnerPasswordState,
    pub reverted_owner_pw: OwnerPasswordState,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[layout(round = 16)]
pub struct RubyDescriptor {
    pub base_com_id: u16,
    pub num_com_ids: u16,
    #[layout(offset = 4, bits = 7..=7)]
    pub no_range_crossing: bool,
    #[layout(offset = 5)]
    pub num_locking_admins_supported: u16,
    pub num_locking_users_supported: u16,
    pub initial_owner_pw: OwnerPasswordState,
    pub reverted_owner_pw: OwnerPasswordState,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[layout(round = 48)]
pub struct KeyPerIODescriptor {
    pub base_com_id_p1: u16,
    pub num_com_ids_p1: u16,
    pub base_com_id_p3: u16,
    pub num_com_ids_p3: u16,
    #[layout(offset = 8)]
    pub initial_owner_pw: OwnerPasswordState,
    pub reverted_owner_pw: OwnerPasswordState,
    pub num_kpio_admins_supported: u16,
    #[layout(offset = 12, bits = 2..=2)]
    pub replay_protection_enabled: bool,
    #[layout(offset = 12, bits = 3..=3)]
    pub replay_protection_supported: bool,
    #[layout(offset = 12, bits = 4..=4)]
    pub incorrect_key_detection_supported: bool,
    #[layout(offset = 12, bits = 5..=5)]
    pub tweak_key_required: bool,
    #[layout(offset = 12, bits = 6..=6)]
    pub kpio_scope: bool,
    #[layout(offset = 12, bits = 7..=7)]
    pub kpio_enabled: bool,
    #[layout(offset = 13)]
    pub max_key_uid_len: u16,
    #[layout(offset = 15, bits = 7..=7)]
    pub kmip_key_injection_supported: bool,
    #[layout(offset = 17, bits = 5..=5)]
    pub nist_rsa_oaep_supported: bool,
    #[layout(offset = 17, bits = 6..=6)]
    pub nist_aes_gcm_supported: bool,
    #[layout(offset = 17, bits = 7..=7)]
    pub nist_aes_kw_supported: bool,
    #[layout(offset = 19, bits = 7..=7)]
    pub rsa2k_wrapping_supported: bool,
    #[layout(offset = 21, bits = 5..=5)]
    pub aes256_wrapping_supported: bool,
    #[layout(offset = 21, bits = 6..=6)]
    pub rsa3k_wrapping_supported: bool,
    #[layout(offset = 21, bits = 7..=7)]
    pub rsa4k_wrapping_supported: bool,
    #[layout(offset = 23, bits = 6..=6)]
    pub pki_kek_transport_supported: bool,
    #[layout(offset = 23, bits = 7..=7)]
    pub plaintext_kek_prov_supported: bool,
    #[layout(offset = 28)]
    pub num_keks_supported: u32,
    pub total_key_tags_supported: u32,
    pub max_key_tags_per_namespace: u16,
    pub get_nonce_cmd_nonce_len: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnrecognizedDescriptor {
    pub feature_code: u16,
    pub version: u8,
    pub length: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FeatureDescriptor {
    TPer(TPerDescriptor),
    Locking(LockingDescriptor),
    Geometry(GeometryDescriptor),
    DataRemoval(DataRemovalDescriptor),
    BlockSIDAuth(BlockSIDAuthDescriptor),
    AdditionalDataStoreTables(AdditionalDataStoreTablesDescriptor),
    Enterprise(EnterpriseDescriptor),
    OpalV1(OpalV1Descriptor),
    OpalV2(OpalV2Descriptor),
    Opalite(OpaliteDescriptor),
    PyriteV1(PyriteV1Descriptor),
    PyriteV2(PyriteV2Descriptor),
    Ruby(RubyDescriptor),
    KeyPerIO(KeyPerIODescriptor),
    Unrecognized(UnrecognizedDescriptor),
}

#[derive(Serialize, Deserialize, Clone)]
#[layout(round = 4)]
struct RawFeatureDescriptor {
    feature_code: FeatureCode,
    #[layout(offset = 2, bits = 4..=7)]
    version: u8,
    #[layout(offset = 3)]
    payload: VecWithLen<u8, u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[layout(round = 48)]
pub struct DiscoveryHeader {
    pub length_of_data: u32,
    pub major_version: u16,
    pub minor_version: u16,
    #[layout(offset = 16)]
    pub vendor_unique: [u8; 32],
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Discovery {
    feature_descriptors: VecWithLen<FeatureDescriptor, DiscoveryHeader>,
}

impl Discovery {
    pub fn new(feature_descriptors: Vec<FeatureDescriptor>) -> Discovery {
        Discovery { feature_descriptors: feature_descriptors.into() }
    }

    pub fn get<'me, T>(&'me self) -> Option<&'me T>
    where
        &'me T: TryFrom<&'me FeatureDescriptor>,
    {
        self.feature_descriptors
            .iter()
            .map(|desc| <&'me FeatureDescriptor as TryInto<&'me T>>::try_into(desc))
            .find_map(|result| result.ok())
    }

    pub fn remove_empty(self) -> Discovery {
        let feature_descriptors: Vec<_> = self
            .feature_descriptors
            .into_iter()
            .filter(|desc| {
                desc != &FeatureDescriptor::Unrecognized(UnrecognizedDescriptor {
                    feature_code: 0,
                    length: 0,
                    version: 0,
                })
            })
            .collect();
        Self { feature_descriptors: feature_descriptors.into() }
    }

    pub fn get_common_features(&self) -> impl Iterator<Item = &FeatureDescriptor> {
        self.feature_descriptors.iter().filter(|desc| desc.security_subsystem_class().is_none())
    }

    pub fn get_ssc_features(&self) -> impl Iterator<Item = &dyn SecuritySubsystemClass> {
        self.feature_descriptors.iter().filter_map(|desc| desc.security_subsystem_class())
    }

    pub fn get_primary_ssc(&self) -> Option<&dyn SecuritySubsystemClass> {
        self.get_ssc_features().next()
    }
}

impl IntoIterator for Discovery {
    type Item = <VecWithLen<FeatureDescriptor, DiscoveryHeader> as IntoIterator>::Item;
    type IntoIter = <VecWithLen<FeatureDescriptor, DiscoveryHeader> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.feature_descriptors.into_iter()
    }
}

impl Deref for Discovery {
    type Target = Vec<FeatureDescriptor>;
    fn deref(&self) -> &Self::Target {
        self.feature_descriptors.deref()
    }
}

impl DerefMut for Discovery {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.feature_descriptors.deref_mut()
    }
}

macro_rules! impl_feature {
    ($desc:path, $feature_code:expr, $version:expr) => {
        impl Feature for $desc {
            fn feature_code(&self) -> FeatureCode {
                $feature_code
            }
            fn version(&self) -> u8 {
                $version
            }
        }
    };
}

macro_rules! impl_security_subsystem_class {
    ($desc:path) => {
        impl SecuritySubsystemClass for $desc {
            fn base_com_id(&self) -> u16 {
                self.base_com_id
            }
            fn num_com_ids(&self) -> u16 {
                self.num_com_ids
            }
        }
    };
}

impl_feature!(TPerDescriptor, FeatureCode::TPer, 1);
impl_feature!(LockingDescriptor, FeatureCode::Locking, 1);
impl_feature!(GeometryDescriptor, FeatureCode::Geometry, 1);
impl_feature!(DataRemovalDescriptor, FeatureCode::DataRemoval, 1);
impl_feature!(BlockSIDAuthDescriptor, FeatureCode::BlockSIDAuth, 1);
impl_feature!(AdditionalDataStoreTablesDescriptor, FeatureCode::AdditionalDataStoreTables, 1);
impl_feature!(EnterpriseDescriptor, FeatureCode::Enterprise, 1);
impl_feature!(KeyPerIODescriptor, FeatureCode::KeyPerIO, 1);
impl_feature!(OpalV1Descriptor, FeatureCode::OpalV1, 1);
impl_feature!(OpalV2Descriptor, FeatureCode::OpalV2, 1);
impl_feature!(OpaliteDescriptor, FeatureCode::Opalite, 1);
impl_feature!(PyriteV1Descriptor, FeatureCode::PyriteV1, 1);
impl_feature!(PyriteV2Descriptor, FeatureCode::PyriteV2, 1);
impl_feature!(RubyDescriptor, FeatureCode::Ruby, 1);

impl_security_subsystem_class!(EnterpriseDescriptor);
impl_security_subsystem_class!(OpalV1Descriptor);
impl_security_subsystem_class!(OpalV2Descriptor);
impl_security_subsystem_class!(OpaliteDescriptor);
impl_security_subsystem_class!(PyriteV1Descriptor);
impl_security_subsystem_class!(PyriteV2Descriptor);
impl_security_subsystem_class!(RubyDescriptor);

impl SecuritySubsystemClass for KeyPerIODescriptor {
    fn base_com_id(&self) -> u16 {
        self.base_com_id_p1
    }
    fn num_com_ids(&self) -> u16 {
        self.num_com_ids_p1
    }
    fn base_com_id_p3(&self) -> Option<u16> {
        Some(self.base_com_id_p3)
    }
    fn num_com_ids_p3(&self) -> Option<u16> {
        Some(self.num_com_ids_p3)
    }
}

impl Feature for FeatureDescriptor {
    fn feature_code(&self) -> FeatureCode {
        match self {
            FeatureDescriptor::TPer(desc) => desc.feature_code(),
            FeatureDescriptor::Locking(desc) => desc.feature_code(),
            FeatureDescriptor::Geometry(desc) => desc.feature_code(),
            FeatureDescriptor::DataRemoval(desc) => desc.feature_code(),
            FeatureDescriptor::BlockSIDAuth(desc) => desc.feature_code(),
            FeatureDescriptor::AdditionalDataStoreTables(desc) => desc.feature_code(),
            FeatureDescriptor::Enterprise(desc) => desc.feature_code(),
            FeatureDescriptor::OpalV1(desc) => desc.feature_code(),
            FeatureDescriptor::OpalV2(desc) => desc.feature_code(),
            FeatureDescriptor::Opalite(desc) => desc.feature_code(),
            FeatureDescriptor::PyriteV1(desc) => desc.feature_code(),
            FeatureDescriptor::PyriteV2(desc) => desc.feature_code(),
            FeatureDescriptor::Ruby(desc) => desc.feature_code(),
            FeatureDescriptor::KeyPerIO(desc) => desc.feature_code(),
            FeatureDescriptor::Unrecognized(_) => FeatureCode::Unrecognized,
        }
    }
    fn version(&self) -> u8 {
        match self {
            FeatureDescriptor::TPer(desc) => desc.version(),
            FeatureDescriptor::Locking(desc) => desc.version(),
            FeatureDescriptor::Geometry(desc) => desc.version(),
            FeatureDescriptor::DataRemoval(desc) => desc.version(),
            FeatureDescriptor::BlockSIDAuth(desc) => desc.version(),
            FeatureDescriptor::AdditionalDataStoreTables(desc) => desc.version(),
            FeatureDescriptor::Enterprise(desc) => desc.version(),
            FeatureDescriptor::OpalV1(desc) => desc.version(),
            FeatureDescriptor::OpalV2(desc) => desc.version(),
            FeatureDescriptor::Opalite(desc) => desc.version(),
            FeatureDescriptor::PyriteV1(desc) => desc.version(),
            FeatureDescriptor::PyriteV2(desc) => desc.version(),
            FeatureDescriptor::Ruby(desc) => desc.version(),
            FeatureDescriptor::KeyPerIO(desc) => desc.version(),
            FeatureDescriptor::Unrecognized(desc) => desc.version,
        }
    }
}

impl FeatureDescriptor {
    pub fn security_subsystem_class(&self) -> Option<&dyn SecuritySubsystemClass> {
        match self {
            FeatureDescriptor::KeyPerIO(desc) => Some(desc as &dyn SecuritySubsystemClass),
            FeatureDescriptor::Enterprise(desc) => Some(desc as &dyn SecuritySubsystemClass),
            FeatureDescriptor::OpalV1(desc) => Some(desc as &dyn SecuritySubsystemClass),
            FeatureDescriptor::OpalV2(desc) => Some(desc as &dyn SecuritySubsystemClass),
            FeatureDescriptor::Opalite(desc) => Some(desc as &dyn SecuritySubsystemClass),
            FeatureDescriptor::PyriteV1(desc) => Some(desc as &dyn SecuritySubsystemClass),
            FeatureDescriptor::PyriteV2(desc) => Some(desc as &dyn SecuritySubsystemClass),
            FeatureDescriptor::Ruby(desc) => Some(desc as &dyn SecuritySubsystemClass),
            _ => None,
        }
    }
}

macro_rules! impl_desc_try_from {
    ($desc:ty, $variant:ident) => {
        impl TryFrom<FeatureDescriptor> for $desc {
            type Error = FeatureDescriptor;
            fn try_from(value: FeatureDescriptor) -> Result<Self, Self::Error> {
                match value {
                    FeatureDescriptor::$variant(desc) => Ok(desc),
                    _ => Err(value),
                }
            }
        }

        impl<'src> TryFrom<&'src FeatureDescriptor> for &'src $desc {
            type Error = &'src FeatureDescriptor;
            fn try_from(value: &'src FeatureDescriptor) -> Result<Self, Self::Error> {
                match value {
                    FeatureDescriptor::$variant(desc) => Ok(desc),
                    _ => Err(value),
                }
            }
        }
    };
}

impl_desc_try_from!(TPerDescriptor, TPer);
impl_desc_try_from!(LockingDescriptor, Locking);
impl_desc_try_from!(OpalV2Descriptor, OpalV2);

impl Serialize<u8> for FeatureDescriptor {
    type Error = SerializeError;
    fn serialize(&self, stream: &mut OutputStream<u8>) -> Result<(), Self::Error> {
        let mut raw_stream = OutputStream::<u8>::new();
        match self {
            FeatureDescriptor::TPer(desc) => desc.serialize(&mut raw_stream),
            FeatureDescriptor::Locking(desc) => desc.serialize(&mut raw_stream),
            FeatureDescriptor::Geometry(desc) => desc.serialize(&mut raw_stream),
            FeatureDescriptor::DataRemoval(desc) => desc.serialize(&mut raw_stream),
            FeatureDescriptor::BlockSIDAuth(desc) => desc.serialize(&mut raw_stream),
            FeatureDescriptor::AdditionalDataStoreTables(desc) => desc.serialize(&mut raw_stream),
            FeatureDescriptor::OpalV2(desc) => desc.serialize(&mut raw_stream),
            FeatureDescriptor::Enterprise(desc) => desc.serialize(&mut raw_stream),
            FeatureDescriptor::OpalV1(desc) => desc.serialize(&mut raw_stream),
            FeatureDescriptor::Opalite(desc) => desc.serialize(&mut raw_stream),
            FeatureDescriptor::PyriteV1(desc) => desc.serialize(&mut raw_stream),
            FeatureDescriptor::PyriteV2(desc) => desc.serialize(&mut raw_stream),
            FeatureDescriptor::Ruby(desc) => desc.serialize(&mut raw_stream),
            FeatureDescriptor::KeyPerIO(desc) => desc.serialize(&mut raw_stream),
            FeatureDescriptor::Unrecognized(_) => Ok(()),
        }?;
        let raw = RawFeatureDescriptor {
            feature_code: self.feature_code(),
            version: self.version(),
            payload: raw_stream.take().into(),
        };
        raw.serialize(stream)
    }
}

impl Deserialize<u8> for FeatureDescriptor {
    type Error = SerializeError;
    fn deserialize(stream: &mut crate::serialization::InputStream<u8>) -> Result<Self, Self::Error> {
        let raw_feature_code = stream.peek_exact(2).map(|slc| u16::from_be_bytes(slc.try_into().unwrap()));
        let raw = RawFeatureDescriptor::deserialize(stream)?;
        let len = raw.payload.len();
        let mut raw_stream = InputStream::from(raw.payload.into_vec());
        let desc = match raw.feature_code {
            FeatureCode::TPer => FeatureDescriptor::TPer(TPerDescriptor::deserialize(&mut raw_stream)?),
            FeatureCode::Locking => FeatureDescriptor::Locking(LockingDescriptor::deserialize(&mut raw_stream)?),
            FeatureCode::Geometry => FeatureDescriptor::Geometry(GeometryDescriptor::deserialize(&mut raw_stream)?),
            FeatureCode::DataRemoval => {
                FeatureDescriptor::DataRemoval(DataRemovalDescriptor::deserialize(&mut raw_stream)?)
            }
            FeatureCode::BlockSIDAuth => {
                FeatureDescriptor::BlockSIDAuth(BlockSIDAuthDescriptor::deserialize(&mut raw_stream)?)
            }
            FeatureCode::AdditionalDataStoreTables => FeatureDescriptor::AdditionalDataStoreTables(
                AdditionalDataStoreTablesDescriptor::deserialize(&mut raw_stream)?,
            ),
            FeatureCode::OpalV2 => FeatureDescriptor::OpalV2(OpalV2Descriptor::deserialize(&mut raw_stream)?),
            FeatureCode::Enterprise => {
                FeatureDescriptor::Enterprise(EnterpriseDescriptor::deserialize(&mut raw_stream)?)
            }
            FeatureCode::OpalV1 => FeatureDescriptor::OpalV1(OpalV1Descriptor::deserialize(&mut raw_stream)?),
            FeatureCode::Opalite => FeatureDescriptor::Opalite(OpaliteDescriptor::deserialize(&mut raw_stream)?),
            FeatureCode::PyriteV1 => FeatureDescriptor::PyriteV1(PyriteV1Descriptor::deserialize(&mut raw_stream)?),
            FeatureCode::PyriteV2 => FeatureDescriptor::PyriteV2(PyriteV2Descriptor::deserialize(&mut raw_stream)?),
            FeatureCode::Ruby => FeatureDescriptor::Ruby(RubyDescriptor::deserialize(&mut raw_stream)?),
            FeatureCode::KeyPerIO => FeatureDescriptor::KeyPerIO(KeyPerIODescriptor::deserialize(&mut raw_stream)?),
            FeatureCode::Unrecognized => FeatureDescriptor::Unrecognized(UnrecognizedDescriptor {
                feature_code: raw_feature_code.unwrap_or(FeatureCode::Unrecognized as u16),
                version: raw.version,
                length: len as u8,
            }),
        };
        Ok(desc)
    }
}

impl TryFrom<DiscoveryHeader> for usize {
    type Error = <usize as TryFrom<u32>>::Error;
    fn try_from(value: DiscoveryHeader) -> Result<Self, Self::Error> {
        Self::try_from(value.length_of_data)
    }
}

impl TryFrom<usize> for DiscoveryHeader {
    type Error = <u32 as TryFrom<usize>>::Error;
    fn try_from(value: usize) -> Result<DiscoveryHeader, Self::Error> {
        let length_of_data: u32 = value.try_into()?;
        Ok(DiscoveryHeader {
            length_of_data: length_of_data,
            major_version: 0,
            minor_version: 1,
            vendor_unique: [0; 32],
        })
    }
}

fn removal_time(format_bit: bool, amount: u16) -> Option<Duration> {
    if amount == 0 {
        None
    } else {
        if format_bit {
            Some(Duration::from_secs(amount as u64 * 2 * 60))
        } else {
            Some(Duration::from_secs(amount as u64 * 2))
        }
    }
}

impl DataRemovalTime {
    pub fn overwrite(&self) -> Option<Duration> {
        removal_time(self.overwrite_unit, self.overwrite_amount)
    }
    pub fn block_erase(&self) -> Option<Duration> {
        removal_time(self.block_erase_unit, self.block_erase_amount)
    }
    pub fn crypto_erase(&self) -> Option<Duration> {
        removal_time(self.crypto_erase_unit, self.crypto_erase_amount)
    }
    pub fn vendor_erase(&self) -> Option<Duration> {
        removal_time(self.vendor_erase_unit, self.vendor_erase_amount)
    }
}

impl core::fmt::Display for FeatureCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FeatureCode::TPer => write!(f, "TPer"),
            FeatureCode::Locking => write!(f, "Locking"),
            FeatureCode::Geometry => write!(f, "Geometry"),
            FeatureCode::DataRemoval => write!(f, "Data removal"),
            FeatureCode::BlockSIDAuth => write!(f, "Block SID authentication"),
            FeatureCode::AdditionalDataStoreTables => write!(f, "Additional DataStore tables"),
            FeatureCode::Enterprise => write!(f, "Enterprise"),
            FeatureCode::OpalV1 => write!(f, "Opal 1.0"),
            FeatureCode::OpalV2 => write!(f, "Opal 2.0"),
            FeatureCode::Opalite => write!(f, "Opalite"),
            FeatureCode::PyriteV1 => write!(f, "Pyrite 1.0"),
            FeatureCode::PyriteV2 => write!(f, "Pyrite 2.0"),
            FeatureCode::Ruby => write!(f, "Ruby"),
            FeatureCode::KeyPerIO => write!(f, "Key per I/O"),
            FeatureCode::Unrecognized => write!(f, "Unrecognized"),
        }
    }
}
