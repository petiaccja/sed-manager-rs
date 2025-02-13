use std::time::Duration;

use crate::serialization::{
    vec_with_len::VecWithLen, Deserialize, Error as SerializeError, InputStream, OutputStream, Serialize,
};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum FeatureCode {
    TPer = 0x0001,
    Locking = 0x0002,
    Geometry = 0x0003,
    DataRemoval = 0x0404,
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

pub struct SSCDescriptor {
    pub base_com_id: u16,
    pub num_com_ids: u16,
}

pub trait Feature {
    fn static_feature_code() -> FeatureCode;
    fn static_version() -> u8;
    fn feature_code(&self) -> FeatureCode {
        Self::static_feature_code()
    }
    fn version(&self) -> u8 {
        Self::static_version()
    }
    fn ssc_desc(&self) -> Option<SSCDescriptor> {
        None
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[layout(round = 12)]
pub struct TPerDescriptor {
    #[layout(offset = 0, bits = 0..=0)]
    pub sync_supported: bool,
    #[layout(offset = 0, bits = 1..=1)]
    pub async_supported: bool,
    #[layout(offset = 0, bits = 2..=2)]
    pub ack_nak_supported: bool,
    #[layout(offset = 0, bits = 3..=3)]
    pub buffer_mgmt_supported: bool,
    #[layout(offset = 0, bits = 4..=4)]
    pub streaming_supported: bool,
    #[layout(offset = 0, bits = 6..=6)]
    pub com_id_mgmt_supported: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[layout(round = 12)]
pub struct LockingDescriptor {
    #[layout(offset = 0, bits = 0..=0)]
    pub locking_supported: bool,
    #[layout(offset = 0, bits = 1..=1)]
    pub locking_enabled: bool,
    #[layout(offset = 0, bits = 2..=2)]
    pub locked: bool,
    #[layout(offset = 0, bits = 3..=3)]
    pub media_encryption: bool,
    #[layout(offset = 0, bits = 4..=4)]
    pub mbr_enabled: bool,
    #[layout(offset = 0, bits = 5..=5)]
    pub mbr_done: bool,
    #[layout(offset = 0, bits = 6..=6)]
    pub mbr_shadowing_not_supported: bool,
    #[layout(offset = 0, bits = 7..=7)]
    pub hw_reset_supported: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct GeometryDescriptor {
    #[layout(offset = 0, bits = 0..=0)]
    pub align: bool,
    #[layout(offset = 8)]
    pub logical_block_size: u32,
    pub alignment_granularity: u64,
    pub lowest_aligned_lba: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DataRemovalMechanism {
    #[layout(offset = 2, bits = 0..=0)]
    pub overwrite: bool,
    #[layout(offset = 2, bits = 1..=1)]
    pub block_erase: bool,
    #[layout(offset = 2, bits = 2..=2)]
    pub crypto_erase: bool,
    #[layout(offset = 2, bits = 5..=5)]
    pub vendor_erase: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DataRemovalTime {
    #[layout(offset = 0, bits = 0..=0)]
    pub overwrite_unit: bool,
    #[layout(offset = 0, bits = 1..=1)]
    pub block_erase_unit: bool,
    #[layout(offset = 0, bits = 2..=2)]
    pub crypto_erase_unit: bool,
    #[layout(offset = 0, bits = 5..=5)]
    pub vendor_erase_unit: bool,
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
    #[layout(offset = 1, bits = 0..=0)]
    pub processing: bool,
    #[layout(offset = 1, bits = 1..=1)]
    pub interrupted: bool,
    #[layout(offset = 2)]
    pub supported_mechanism: DataRemovalMechanism,
    pub erase_time_unit: DataRemovalMechanism,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[layout(round = 16)]
pub struct EnterpriseDescriptor {
    pub base_com_id: u16,
    pub num_com_ids: u16,
    #[layout(offset = 4, bits = 0..=0)]
    pub no_range_crossing: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[layout(round = 16)]
pub struct OpalV1Descriptor {
    pub base_com_id: u16,
    pub num_com_ids: u16,
    #[layout(offset = 4, bits = 0..=0)]
    pub no_range_crossing: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[layout(round = 16)]
pub struct OpalV2Descriptor {
    pub base_com_id: u16,
    pub num_com_ids: u16,
    #[layout(offset = 4, bits = 0..=0)]
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
    #[layout(offset = 4, bits = 0..=0)]
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
    #[layout(offset = 12, bits = 0..=0)]
    pub kpio_enabled: bool,
    #[layout(offset = 12, bits = 1..=1)]
    pub kpio_scope: bool,
    #[layout(offset = 12, bits = 2..=2)]
    pub tweak_key_required: bool,
    #[layout(offset = 12, bits = 3..=3)]
    pub incorrect_key_detection_supported: bool,
    #[layout(offset = 12, bits = 4..=4)]
    pub replay_protection_supported: bool,
    #[layout(offset = 12, bits = 5..=5)]
    pub replay_protection_enabled: bool,
    #[layout(offset = 13)]
    pub max_key_uid_len: u16,
    #[layout(offset = 15, bits = 0..=0)]
    pub kmip_key_injection_supported: bool,
    #[layout(offset = 17, bits = 0..=0)]
    pub nist_aes_kw_supported: bool,
    #[layout(offset = 17, bits = 1..=1)]
    pub nist_aes_gcm_supported: bool,
    #[layout(offset = 17, bits = 2..=2)]
    pub nist_rsa_oaep_supported: bool,
    #[layout(offset = 19, bits = 0..=0)]
    pub aes256_wrapping_supported: bool,
    #[layout(offset = 21, bits = 0..=0)]
    pub rsa2k_wrapping_supported: bool,
    #[layout(offset = 21, bits = 1..=1)]
    pub rsa3k_wrapping_supported: bool,
    #[layout(offset = 21, bits = 2..=2)]
    pub rsa4k_wrapping_supported: bool,
    #[layout(offset = 23, bits = 0..=0)]
    pub plaintext_kek_prov_supported: bool,
    #[layout(offset = 23, bits = 1..=1)]
    pub pki_kek_transport_supported: bool,
    #[layout(offset = 28)]
    pub num_keks_supported: u32,
    pub total_key_tags_supported: u32,
    pub max_key_tags_per_namespace: u16,
    pub get_nonce_cmd_nonce_len: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FeatureDescriptor {
    TPer(TPerDescriptor),
    Locking(LockingDescriptor),
    Geometry(GeometryDescriptor),
    DataRemoval(DataRemovalDescriptor),
    Enterprise(EnterpriseDescriptor),
    OpalV1(OpalV1Descriptor),
    OpalV2(OpalV2Descriptor),
    Opalite(OpaliteDescriptor),
    PyriteV1(PyriteV1Descriptor),
    PyriteV2(PyriteV2Descriptor),
    Ruby(RubyDescriptor),
    KeyPerIO(KeyPerIODescriptor),
    Unrecognized,
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
    pub descriptors: VecWithLen<FeatureDescriptor, DiscoveryHeader>,
}

impl Discovery {
    pub fn new(descs: Vec<FeatureDescriptor>) -> Discovery {
        Discovery { descriptors: VecWithLen::from(descs) }
    }

    pub fn get<'me, T>(&'me self) -> Option<&'me T>
    where
        &'me T: TryFrom<&'me FeatureDescriptor>,
    {
        self.descriptors
            .iter()
            .map(|desc| <&'me FeatureDescriptor as TryInto<&'me T>>::try_into(desc))
            .find_map(|result| result.ok())
    }
}

macro_rules! impl_feature {
    ($desc:path, $feature_code:expr, $version:expr) => {
        impl Feature for $desc {
            fn static_feature_code() -> FeatureCode {
                $feature_code
            }
            fn static_version() -> u8 {
                $version
            }
        }
    };
}

macro_rules! impl_ssc_feature {
    ($desc:path, $feature_code:expr, $version:expr) => {
        impl Feature for $desc {
            fn static_feature_code() -> FeatureCode {
                $feature_code
            }
            fn static_version() -> u8 {
                $version
            }
            fn ssc_desc(&self) -> Option<SSCDescriptor> {
                Some(SSCDescriptor { base_com_id: self.base_com_id, num_com_ids: self.num_com_ids })
            }
        }
    };
}

impl_feature!(TPerDescriptor, FeatureCode::TPer, 1);
impl_feature!(LockingDescriptor, FeatureCode::Locking, 1);
impl_feature!(GeometryDescriptor, FeatureCode::Geometry, 1);
impl_feature!(DataRemovalDescriptor, FeatureCode::DataRemoval, 1);
impl_ssc_feature!(EnterpriseDescriptor, FeatureCode::Enterprise, 1);
impl_ssc_feature!(OpalV1Descriptor, FeatureCode::OpalV1, 1);
impl_ssc_feature!(OpalV2Descriptor, FeatureCode::OpalV2, 1);
impl_ssc_feature!(OpaliteDescriptor, FeatureCode::Opalite, 1);
impl_ssc_feature!(PyriteV1Descriptor, FeatureCode::PyriteV1, 1);
impl_ssc_feature!(PyriteV2Descriptor, FeatureCode::PyriteV2, 1);
impl_ssc_feature!(RubyDescriptor, FeatureCode::Ruby, 1);

impl Feature for KeyPerIODescriptor {
    fn static_feature_code() -> FeatureCode {
        FeatureCode::KeyPerIO
    }
    fn static_version() -> u8 {
        1
    }
    fn ssc_desc(&self) -> Option<SSCDescriptor> {
        Some(SSCDescriptor { base_com_id: self.base_com_id_p1, num_com_ids: self.num_com_ids_p1 })
    }
}

impl FeatureDescriptor {
    pub fn feature_code(&self) -> FeatureCode {
        match self {
            FeatureDescriptor::TPer(desc) => desc.feature_code(),
            FeatureDescriptor::Locking(desc) => desc.feature_code(),
            FeatureDescriptor::Geometry(desc) => desc.feature_code(),
            FeatureDescriptor::DataRemoval(desc) => desc.feature_code(),
            FeatureDescriptor::OpalV2(desc) => desc.feature_code(),
            FeatureDescriptor::Unrecognized => FeatureCode::Unrecognized,
            FeatureDescriptor::Enterprise(desc) => desc.feature_code(),
            FeatureDescriptor::OpalV1(desc) => desc.feature_code(),
            FeatureDescriptor::Opalite(desc) => desc.feature_code(),
            FeatureDescriptor::PyriteV1(desc) => desc.feature_code(),
            FeatureDescriptor::PyriteV2(desc) => desc.feature_code(),
            FeatureDescriptor::Ruby(desc) => desc.feature_code(),
            FeatureDescriptor::KeyPerIO(desc) => desc.feature_code(),
        }
    }
    pub fn version(&self) -> u8 {
        match self {
            FeatureDescriptor::TPer(desc) => desc.version(),
            FeatureDescriptor::Locking(desc) => desc.version(),
            FeatureDescriptor::Geometry(desc) => desc.version(),
            FeatureDescriptor::DataRemoval(desc) => desc.version(),
            FeatureDescriptor::OpalV2(desc) => desc.version(),
            FeatureDescriptor::Unrecognized => 1,
            FeatureDescriptor::Enterprise(desc) => desc.version(),
            FeatureDescriptor::OpalV1(desc) => desc.version(),
            FeatureDescriptor::Opalite(desc) => desc.version(),
            FeatureDescriptor::PyriteV1(desc) => desc.version(),
            FeatureDescriptor::PyriteV2(desc) => desc.version(),
            FeatureDescriptor::Ruby(desc) => desc.version(),
            FeatureDescriptor::KeyPerIO(desc) => desc.version(),
        }
    }
    pub fn ssc_desc(&self) -> Option<SSCDescriptor> {
        match self {
            FeatureDescriptor::TPer(desc) => desc.ssc_desc(),
            FeatureDescriptor::Locking(desc) => desc.ssc_desc(),
            FeatureDescriptor::Geometry(desc) => desc.ssc_desc(),
            FeatureDescriptor::DataRemoval(desc) => desc.ssc_desc(),
            FeatureDescriptor::OpalV2(desc) => desc.ssc_desc(),
            FeatureDescriptor::Unrecognized => None,
            FeatureDescriptor::Enterprise(desc) => desc.ssc_desc(),
            FeatureDescriptor::OpalV1(desc) => desc.ssc_desc(),
            FeatureDescriptor::Opalite(desc) => desc.ssc_desc(),
            FeatureDescriptor::PyriteV1(desc) => desc.ssc_desc(),
            FeatureDescriptor::PyriteV2(desc) => desc.ssc_desc(),
            FeatureDescriptor::Ruby(desc) => desc.ssc_desc(),
            FeatureDescriptor::KeyPerIO(desc) => desc.ssc_desc(),
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
            FeatureDescriptor::OpalV2(desc) => desc.serialize(&mut raw_stream),
            FeatureDescriptor::Unrecognized => Ok(()),
            FeatureDescriptor::Enterprise(desc) => desc.serialize(&mut raw_stream),
            FeatureDescriptor::OpalV1(desc) => desc.serialize(&mut raw_stream),
            FeatureDescriptor::Opalite(desc) => desc.serialize(&mut raw_stream),
            FeatureDescriptor::PyriteV1(desc) => desc.serialize(&mut raw_stream),
            FeatureDescriptor::PyriteV2(desc) => desc.serialize(&mut raw_stream),
            FeatureDescriptor::Ruby(desc) => desc.serialize(&mut raw_stream),
            FeatureDescriptor::KeyPerIO(desc) => desc.serialize(&mut raw_stream),
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
        let raw = RawFeatureDescriptor::deserialize(stream)?;
        let mut raw_stream = InputStream::from(raw.payload.into_vec());
        let desc = match raw.feature_code {
            FeatureCode::TPer => FeatureDescriptor::TPer(TPerDescriptor::deserialize(&mut raw_stream)?),
            FeatureCode::Locking => FeatureDescriptor::Locking(LockingDescriptor::deserialize(&mut raw_stream)?),
            FeatureCode::Geometry => FeatureDescriptor::Geometry(GeometryDescriptor::deserialize(&mut raw_stream)?),
            FeatureCode::DataRemoval => {
                FeatureDescriptor::DataRemoval(DataRemovalDescriptor::deserialize(&mut raw_stream)?)
            }
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
            _ => FeatureDescriptor::Unrecognized,
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
