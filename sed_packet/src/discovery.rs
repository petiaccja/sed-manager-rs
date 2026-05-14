//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::time::Duration;
use std::{marker::PhantomData, ops::Range};

use sorbit::{
    Deserialize, Serialize,
    byte_order::ByteOrder,
    collection,
    ser_de::{Deserialize, MultiPassSerialize, RevisableSerializer, Serialize, Span},
};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
#[sorbit(byte_order=big_endian)]
pub enum OwnerPasswordState {
    SameAsMSID = 0x00,
    VendorSpecified = 0xFF,
}

/// The shared properties of feature descriptors in a discovery message.
pub trait Feature {
    /// The name of the feature (i.e. Opal 2.0).
    fn name(&self) -> &'static str;

    /// The (major) version of the feature descriptor (not the feature).
    fn version(&self) -> u8;

    /// The minor version of the feature descriptor. Zero if not present.
    fn minor_version(&self) -> u8 {
        0
    }

    /// Additional properties in case the feature is an SSC.
    fn as_ssc(&self) -> Option<&dyn SecuritySubsystemClass> {
        None
    }
}

/// The shared properties of security subsystem classes.
pub trait SecuritySubsystemClass: Feature {
    /// The range of statically allocated ComIDs for protocol 0x01.
    ///
    /// The range starts at the base ComID and has a length equal to the number
    /// of statically allocated ComIDs specified by the feature descriptor.
    fn static_com_ids_p1(&self) -> Range<u16>;

    /// The range of statically allocated ComIDs for protocol 0x02.
    ///
    /// The range starts at the base ComID and has a length equal to the number
    /// of statically allocated ComIDs specified by the feature descriptor.
    fn static_com_ids_p3(&self) -> Range<u16> {
        0..0
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[sorbit(len=14, byte_order=big_endian)]
pub struct TperDescriptor {
    #[sorbit(bit_field = _ver, repr=u8, bits=4..=7)]
    #[sorbit(value = constant(0x01))]
    pub version: PhantomData<u8>,
    #[sorbit(value = constant(0x0C))]
    pub length: PhantomData<u8>,

    #[sorbit(bit_field=_0, repr=u8, bits=6)]
    pub com_id_mgmt_supported: bool,
    #[sorbit(bit_field=_0, bits=4)]
    pub streaming_supported: bool,
    #[sorbit(bit_field=_0, bits=3)]
    pub buffer_mgmt_supported: bool,
    #[sorbit(bit_field=_0, bits=2)]
    pub ack_nak_supported: bool,
    #[sorbit(bit_field=_0, bits=1)]
    pub async_supported: bool,
    #[sorbit(bit_field=_0, bits=0)]
    pub sync_supported: bool,
}

impl Feature for TperDescriptor {
    fn name(&self) -> &'static str {
        "TPer"
    }

    fn version(&self) -> u8 {
        0x01
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[sorbit(len=14, byte_order=big_endian)]
pub struct LockingDescriptor {
    #[sorbit(bit_field = _ver, repr=u8, bits=4..=7)]
    #[sorbit(value = constant(0x01))]
    pub version: PhantomData<u8>,
    #[sorbit(value = constant(0x0C))]
    pub length: PhantomData<u8>,

    #[sorbit(bit_field=_0, repr=u8, bits=7)]
    pub hw_reset_supported: bool,
    #[sorbit(bit_field=_0, bits=6)]
    pub mbr_shadowing_not_supported: bool,
    #[sorbit(bit_field=_0, bits=5)]
    pub mbr_done: bool,
    #[sorbit(bit_field=_0, bits=4)]
    pub mbr_enabled: bool,
    #[sorbit(bit_field=_0, bits=3)]
    pub media_encryption: bool,
    #[sorbit(bit_field=_0, bits=2)]
    pub locked: bool,
    #[sorbit(bit_field=_0, bits=1)]
    pub locking_enabled: bool,
    #[sorbit(bit_field=_0, bits=0)]
    pub locking_supported: bool,
}

impl Feature for LockingDescriptor {
    fn name(&self) -> &'static str {
        "Locking"
    }

    fn version(&self) -> u8 {
        0x01
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[sorbit(len=30, byte_order=big_endian)]
pub struct GeometryDescriptor {
    #[sorbit(bit_field = _ver, repr=u8, bits=4..=7)]
    #[sorbit(value = constant(0x01))]
    pub version: PhantomData<u8>,
    #[sorbit(value = constant(0x1C))]
    pub length: PhantomData<u8>,

    #[sorbit(bit_field=_0, repr=u8, bits=0)]
    pub align: bool,
    #[sorbit(offset = 10)]
    pub logical_block_size: u32,
    pub alignment_granularity: u64,
    pub lowest_aligned_lba: u64,
}

impl Feature for GeometryDescriptor {
    fn name(&self) -> &'static str {
        "Geometry"
    }

    fn version(&self) -> u8 {
        0x01
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[sorbit(byte_order=big_endian)]
pub struct DataRemovalMechanism {
    #[sorbit(bit_field = _0, repr = u8, bits = 5)]
    pub vendor_erase: bool,
    #[sorbit(bit_field = _0, bits = 2)]
    pub crypto_erase: bool,
    #[sorbit(bit_field = _0, bits = 1)]
    pub block_erase: bool,
    #[sorbit(bit_field = _0, bits = 0)]
    pub overwrite: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[sorbit(byte_order=big_endian)]
pub struct DataRemovalTime {
    #[sorbit(bit_field = _0, repr = u8, bits = 5)]
    pub vendor_erase_unit: bool,
    #[sorbit(bit_field = _0, bits = 2)]
    pub crypto_erase_unit: bool,
    #[sorbit(bit_field = _0, bits = 1)]
    pub block_erase_unit: bool,
    #[sorbit(bit_field = _0, bits = 0)]
    pub overwrite_unit: bool,
    #[sorbit(offset = 1)]
    pub overwrite_amount: u16,
    #[sorbit(offset = 3)]
    pub block_erase_amount: u16,
    #[sorbit(offset = 5)]
    pub crypto_erase_amount: u16,
    #[sorbit(offset = 11)]
    pub vendor_erase_amount: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[sorbit(byte_order=big_endian)]
#[sorbit(len = 34)]
pub struct DataRemovalDescriptor {
    #[sorbit(bit_field = _ver, repr=u8, bits=4..=7)]
    #[sorbit(value = constant(0x02))]
    pub version: PhantomData<u8>,
    #[sorbit(value = constant(0x20))]
    pub length: PhantomData<u8>,

    #[sorbit(bit_field = _0, repr = u8, offset = 3, bits = 1)]
    pub interrupted: bool,
    #[sorbit(bit_field = _0, bits = 0)]
    pub processing: bool,
    pub supported_mechanism: DataRemovalMechanism,
    pub removal_time: DataRemovalTime,
}

impl Feature for DataRemovalDescriptor {
    fn name(&self) -> &'static str {
        "Data removal"
    }

    fn version(&self) -> u8 {
        0x02
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[sorbit(len = 14)]
pub struct BlockSIDAuthDescriptor {
    #[sorbit(bit_field = _ver, repr=u8, bits=4..=7)]
    pub version: u8,
    #[sorbit(value = constant(0x0C))]
    pub length: PhantomData<u8>,

    #[sorbit(bit_field = _0, repr = u8, bits = 3)]
    pub locking_sp_frozen: bool,
    #[sorbit(bit_field = _0,bits = 2)]
    pub locking_sp_freeze_supported: bool,
    #[sorbit( bit_field = _0,  bits = 1)]
    pub sid_authentication_blocked: bool,
    #[sorbit(bit_field = _0,  bits = 0)]
    pub sid_msid_pin_differ: bool,
    #[sorbit(bit_field = _1, repr=u8, bits=0)]
    pub hw_reset_unblocks: bool,
}

impl Feature for BlockSIDAuthDescriptor {
    fn name(&self) -> &'static str {
        "Block SID auth."
    }

    fn version(&self) -> u8 {
        self.version
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[sorbit(len = 14)]
pub struct AdditionalDataStoreTablesDescriptor {
    #[sorbit(bit_field = _ver, repr=u8, bits=4..=7)]
    #[sorbit(value = constant(0x02))]
    pub version: PhantomData<u8>,
    #[sorbit(bit_field = _ver, bits=0..=3)]
    pub minor_version: u8,
    #[sorbit(value = constant(0x0C))]
    pub length: PhantomData<u8>,

    #[sorbit(offset = 4)]
    pub max_num_tables: u16,
    pub max_total_size_of_tables: u32,
    pub table_size_alignment: u32,
}

impl Feature for AdditionalDataStoreTablesDescriptor {
    fn name(&self) -> &'static str {
        "Add. DataStore tables"
    }

    fn version(&self) -> u8 {
        0x02
    }

    fn minor_version(&self) -> u8 {
        self.minor_version
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[sorbit(len = 18)]
pub struct EnterpriseDescriptor {
    #[sorbit(bit_field = _ver, repr=u8, bits=3..=7)]
    pub version: u8,
    #[sorbit(value = constant(0x10))]
    pub length: PhantomData<u8>,

    pub base_com_id: u16,
    pub num_com_ids: u16,
    #[sorbit(bit_field = _0, repr=u8, bits=0)]
    pub no_range_crossing: bool,
}

impl Feature for EnterpriseDescriptor {
    fn name(&self) -> &'static str {
        "Enterprise"
    }

    fn version(&self) -> u8 {
        self.version
    }
}

impl SecuritySubsystemClass for EnterpriseDescriptor {
    fn static_com_ids_p1(&self) -> Range<u16> {
        self.base_com_id..self.base_com_id + self.num_com_ids
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[sorbit(len = 18)]
pub struct OpalV1Descriptor {
    #[sorbit(bit_field = _ver, repr=u8, bits=4..=7)]
    pub version: u8,
    #[sorbit(value = constant(0x10))]
    pub length: PhantomData<u8>,

    pub base_com_id: u16,
    pub num_com_ids: u16,
    #[sorbit(bit_field = _0, repr = u8, bits=0)]
    pub no_range_crossing: bool,
}

impl Feature for OpalV1Descriptor {
    fn name(&self) -> &'static str {
        "Opal 1.0"
    }

    fn version(&self) -> u8 {
        self.version
    }
}

impl SecuritySubsystemClass for OpalV1Descriptor {
    fn static_com_ids_p1(&self) -> Range<u16> {
        self.base_com_id..self.base_com_id + self.num_com_ids
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[sorbit(len = 18)]
pub struct OpalV2Descriptor {
    #[sorbit(bit_field = _ver, repr=u8, bits=4..=7)]
    pub version: u8,
    #[sorbit(value = constant(0x10))]
    pub length: PhantomData<u8>,

    pub base_com_id: u16,
    pub num_com_ids: u16,
    #[sorbit(bit_field = _0, repr=u8, bits=0)]
    pub no_range_crossing: bool,
    pub num_locking_admins_supported: u16,
    pub num_locking_users_supported: u16,
    pub initial_owner_pw: OwnerPasswordState,
    pub reverted_owner_pw: OwnerPasswordState,
}

impl Feature for OpalV2Descriptor {
    fn name(&self) -> &'static str {
        "Opal 2.0"
    }

    fn version(&self) -> u8 {
        self.version
    }
}

impl SecuritySubsystemClass for OpalV2Descriptor {
    fn static_com_ids_p1(&self) -> Range<u16> {
        self.base_com_id..self.base_com_id + self.num_com_ids
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[sorbit(len = 18)]
pub struct OpaliteDescriptor {
    #[sorbit(bit_field = _ver, repr=u8, bits=4..=7)]
    pub version: u8,
    #[sorbit(value = constant(0x10))]
    pub length: PhantomData<u8>,

    pub base_com_id: u16,
    pub num_com_ids: u16,
    #[sorbit(offset = 11)]
    pub initial_owner_pw: OwnerPasswordState,
    pub reverted_owner_pw: OwnerPasswordState,
}

impl Feature for OpaliteDescriptor {
    fn name(&self) -> &'static str {
        "Opalite"
    }

    fn version(&self) -> u8 {
        self.version
    }
}

impl SecuritySubsystemClass for OpaliteDescriptor {
    fn static_com_ids_p1(&self) -> Range<u16> {
        self.base_com_id..self.base_com_id + self.num_com_ids
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[sorbit(len = 18)]
pub struct PyriteV1Descriptor {
    #[sorbit(bit_field = _ver, repr=u8, bits=4..=7)]
    pub version: u8,
    #[sorbit(value = constant(0x10))]
    pub length: PhantomData<u8>,

    pub base_com_id: u16,
    pub num_com_ids: u16,
    #[sorbit(offset = 11)]
    pub initial_owner_pw: OwnerPasswordState,
    pub reverted_owner_pw: OwnerPasswordState,
}

impl Feature for PyriteV1Descriptor {
    fn name(&self) -> &'static str {
        "Pyrite 1.0"
    }

    fn version(&self) -> u8 {
        self.version
    }
}

impl SecuritySubsystemClass for PyriteV1Descriptor {
    fn static_com_ids_p1(&self) -> Range<u16> {
        self.base_com_id..self.base_com_id + self.num_com_ids
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[sorbit(len = 18)]
pub struct PyriteV2Descriptor {
    #[sorbit(bit_field = _ver, repr=u8, bits=4..=7)]
    pub version: u8,
    #[sorbit(value = constant(0x10))]
    pub length: PhantomData<u8>,

    pub base_com_id: u16,
    pub num_com_ids: u16,
    #[sorbit(offset = 11)]
    pub initial_owner_pw: OwnerPasswordState,
    pub reverted_owner_pw: OwnerPasswordState,
}

impl Feature for PyriteV2Descriptor {
    fn name(&self) -> &'static str {
        "Pyrite 2.0"
    }

    fn version(&self) -> u8 {
        self.version
    }
}

impl SecuritySubsystemClass for PyriteV2Descriptor {
    fn static_com_ids_p1(&self) -> Range<u16> {
        self.base_com_id..self.base_com_id + self.num_com_ids
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[sorbit(len = 18)]
pub struct RubyDescriptor {
    #[sorbit(bit_field = _ver, repr=u8, bits=4..=7)]
    pub version: u8,
    #[sorbit(value = constant(0x10))]
    pub length: PhantomData<u8>,

    pub base_com_id: u16,
    pub num_com_ids: u16,
    #[sorbit(bit_field = _0, repr = u8, bits = 0)]
    pub no_range_crossing: bool,
    pub num_locking_admins_supported: u16,
    pub num_locking_users_supported: u16,
    pub initial_owner_pw: OwnerPasswordState,
    pub reverted_owner_pw: OwnerPasswordState,
}

impl Feature for RubyDescriptor {
    fn name(&self) -> &'static str {
        "Ruby"
    }

    fn version(&self) -> u8 {
        self.version
    }
}

impl SecuritySubsystemClass for RubyDescriptor {
    fn static_com_ids_p1(&self) -> Range<u16> {
        self.base_com_id..self.base_com_id + self.num_com_ids
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[sorbit(len = 46)]
pub struct KeyPerIODescriptor {
    #[sorbit(bit_field = _ver, repr=u8, bits=4..=7)]
    pub version: u8,
    #[sorbit(bit_field = _ver, repr=u8, bits=0..=3)]
    pub minor_version: u8,
    #[sorbit(value = constant(0x10))]
    pub length: PhantomData<u8>,

    pub base_com_id_p1: u16,
    pub num_com_ids_p1: u16,
    pub base_com_id_p3: u16,
    pub num_com_ids_p3: u16,
    #[sorbit(offset = 10)]
    pub initial_owner_pw: OwnerPasswordState,
    pub reverted_owner_pw: OwnerPasswordState,
    pub num_kpio_admins_supported: u16,

    #[sorbit(bit_field = _0, repr = u8, bits = 5)]
    pub replay_protection_enabled: bool,
    #[sorbit(bit_field = _0, bits = 4)]
    pub replay_protection_supported: bool,
    #[sorbit(bit_field = _0, bits = 3)]
    pub incorrect_key_detection_supported: bool,
    #[sorbit(bit_field = _0, bits = 2)]
    pub tweak_key_required: bool,
    #[sorbit(bit_field = _0, bits = 1)]
    pub kpio_scope: bool,
    #[sorbit(bit_field = _0, bits = 0)]
    pub kpio_enabled: bool,

    pub max_key_uid_len: u16,

    #[sorbit(offset = 17, bit_field = _1,  repr = u8,bits = 0)]
    pub kmip_key_injection_supported: bool,

    #[sorbit(offset = 19, bit_field = _2, repr = u8, bits = 2)]
    pub nist_rsa_oaep_supported: bool,
    #[sorbit(bit_field = _2, bits = 1)]
    pub nist_aes_gcm_supported: bool,
    #[sorbit(bit_field = _2, bits = 0)]
    pub nist_aes_kw_supported: bool,

    #[sorbit(offset = 21, bit_field = _3, repr = u8, bits = 0)]
    pub aes256_wrapping_supported: bool,

    #[sorbit(offset = 23, bit_field = _4, repr = u8, bits = 0)]
    pub rsa2k_wrapping_supported: bool,
    #[sorbit(bit_field = _4, bits = 1)]
    pub rsa3k_wrapping_supported: bool,
    #[sorbit(bit_field = _4, bits = 2)]
    pub rsa4k_wrapping_supported: bool,

    #[sorbit(offset = 25, bit_field = _5, repr = u8, bits = 1)]
    pub pki_kek_transport_supported: bool,
    #[sorbit(bit_field = _5, bits = 0)]
    pub plaintext_kek_prov_supported: bool,

    #[sorbit(offset = 30)]
    pub num_keks_supported: u32,
    pub total_key_tags_supported: u32,
    pub max_key_tags_per_namespace: u16,
    pub get_nonce_cmd_nonce_len: u8,
}

impl Feature for KeyPerIODescriptor {
    fn name(&self) -> &'static str {
        "Key per I/O"
    }

    fn version(&self) -> u8 {
        self.version
    }
}

impl SecuritySubsystemClass for KeyPerIODescriptor {
    fn static_com_ids_p1(&self) -> Range<u16> {
        self.base_com_id_p1..self.base_com_id_p1 + self.num_com_ids_p1
    }

    fn static_com_ids_p3(&self) -> Range<u16> {
        self.base_com_id_p3..self.base_com_id_p3 + self.num_com_ids_p3
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnrecognizedDescriptor {
    pub version: u8,
    pub length: u8,
}

impl Feature for UnrecognizedDescriptor {
    fn name(&self) -> &'static str {
        "Unrecognized"
    }

    fn version(&self) -> u8 {
        self.version
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[repr(u16)]
#[sorbit(byte_order=big_endian)]
pub enum FeatureDescriptor {
    TPer(TperDescriptor) = 0x0001,
    Locking(LockingDescriptor) = 0x0002,
    Geometry(GeometryDescriptor) = 0x0003,
    DataRemoval(DataRemovalDescriptor) = 0x0404,
    BlockSIDAuth(BlockSIDAuthDescriptor) = 0x0402,
    AdditionalDataStoreTables(AdditionalDataStoreTablesDescriptor) = 0x0202,
    Enterprise(EnterpriseDescriptor) = 0x0100,
    OpalV1(OpalV1Descriptor) = 0x0200,
    OpalV2(OpalV2Descriptor) = 0x0203,
    Opalite(OpaliteDescriptor) = 0x0301,
    PyriteV1(PyriteV1Descriptor) = 0x0302,
    PyriteV2(PyriteV2Descriptor) = 0x0303,
    Ruby(RubyDescriptor) = 0x0304,
    KeyPerIO(KeyPerIODescriptor) = 0x0305,
    #[sorbit(catch_all)]
    Unrecognized(u16, UnrecognizedDescriptor),
}

impl Feature for FeatureDescriptor {
    fn name(&self) -> &'static str {
        match self {
            FeatureDescriptor::TPer(desc) => desc.name(),
            FeatureDescriptor::Locking(desc) => desc.name(),
            FeatureDescriptor::Geometry(desc) => desc.name(),
            FeatureDescriptor::DataRemoval(desc) => desc.name(),
            FeatureDescriptor::BlockSIDAuth(desc) => desc.name(),
            FeatureDescriptor::AdditionalDataStoreTables(desc) => desc.name(),
            FeatureDescriptor::Enterprise(desc) => desc.name(),
            FeatureDescriptor::OpalV1(desc) => desc.name(),
            FeatureDescriptor::OpalV2(desc) => desc.name(),
            FeatureDescriptor::Opalite(desc) => desc.name(),
            FeatureDescriptor::PyriteV1(desc) => desc.name(),
            FeatureDescriptor::PyriteV2(desc) => desc.name(),
            FeatureDescriptor::Ruby(desc) => desc.name(),
            FeatureDescriptor::KeyPerIO(desc) => desc.name(),
            FeatureDescriptor::Unrecognized(_, desc) => desc.name(),
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
            FeatureDescriptor::Unrecognized(_, desc) => desc.version(),
        }
    }

    fn minor_version(&self) -> u8 {
        match self {
            FeatureDescriptor::TPer(desc) => desc.minor_version(),
            FeatureDescriptor::Locking(desc) => desc.minor_version(),
            FeatureDescriptor::Geometry(desc) => desc.minor_version(),
            FeatureDescriptor::DataRemoval(desc) => desc.minor_version(),
            FeatureDescriptor::BlockSIDAuth(desc) => desc.minor_version(),
            FeatureDescriptor::AdditionalDataStoreTables(desc) => desc.minor_version(),
            FeatureDescriptor::Enterprise(desc) => desc.minor_version(),
            FeatureDescriptor::OpalV1(desc) => desc.minor_version(),
            FeatureDescriptor::OpalV2(desc) => desc.minor_version(),
            FeatureDescriptor::Opalite(desc) => desc.minor_version(),
            FeatureDescriptor::PyriteV1(desc) => desc.minor_version(),
            FeatureDescriptor::PyriteV2(desc) => desc.minor_version(),
            FeatureDescriptor::Ruby(desc) => desc.minor_version(),
            FeatureDescriptor::KeyPerIO(desc) => desc.minor_version(),
            FeatureDescriptor::Unrecognized(_, desc) => desc.minor_version(),
        }
    }

    fn as_ssc(&self) -> Option<&dyn SecuritySubsystemClass> {
        match self {
            FeatureDescriptor::TPer(desc) => desc.as_ssc(),
            FeatureDescriptor::Locking(desc) => desc.as_ssc(),
            FeatureDescriptor::Geometry(desc) => desc.as_ssc(),
            FeatureDescriptor::DataRemoval(desc) => desc.as_ssc(),
            FeatureDescriptor::BlockSIDAuth(desc) => desc.as_ssc(),
            FeatureDescriptor::AdditionalDataStoreTables(desc) => desc.as_ssc(),
            FeatureDescriptor::Enterprise(desc) => desc.as_ssc(),
            FeatureDescriptor::OpalV1(desc) => desc.as_ssc(),
            FeatureDescriptor::OpalV2(desc) => desc.as_ssc(),
            FeatureDescriptor::Opalite(desc) => desc.as_ssc(),
            FeatureDescriptor::PyriteV1(desc) => desc.as_ssc(),
            FeatureDescriptor::PyriteV2(desc) => desc.as_ssc(),
            FeatureDescriptor::Ruby(desc) => desc.as_ssc(),
            FeatureDescriptor::KeyPerIO(desc) => desc.as_ssc(),
            FeatureDescriptor::Unrecognized(_, desc) => desc.as_ssc(),
        }
    }
}

macro_rules! impl_convert {
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

impl_convert!(TperDescriptor, TPer);
impl_convert!(LockingDescriptor, Locking);
impl_convert!(GeometryDescriptor, Geometry);
impl_convert!(DataRemovalDescriptor, DataRemoval);
impl_convert!(BlockSIDAuthDescriptor, BlockSIDAuth);
impl_convert!(AdditionalDataStoreTablesDescriptor, AdditionalDataStoreTables);
impl_convert!(OpalV2Descriptor, OpalV2);
impl_convert!(EnterpriseDescriptor, Enterprise);
impl_convert!(OpalV1Descriptor, OpalV1);
impl_convert!(OpaliteDescriptor, Opalite);
impl_convert!(PyriteV1Descriptor, PyriteV1);
impl_convert!(PyriteV2Descriptor, PyriteV2);
impl_convert!(RubyDescriptor, Ruby);
impl_convert!(KeyPerIODescriptor, KeyPerIO);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Discovery {
    pub major_version: u16,
    pub minor_version: u16,
    pub vendor_unique: [u8; 32],
    pub feature_descriptors: Vec<FeatureDescriptor>,
}

impl Discovery {
    pub fn new() -> Discovery {
        Self::default()
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

    pub fn common_features(&self) -> impl Iterator<Item = &FeatureDescriptor> {
        self.feature_descriptors.iter().filter(|desc| desc.as_ssc().is_none())
    }

    pub fn ssc_features(&self) -> impl Iterator<Item = &dyn SecuritySubsystemClass> {
        self.feature_descriptors.iter().filter_map(|desc| desc.as_ssc())
    }

    pub fn primary_ssc(&self) -> Option<&dyn SecuritySubsystemClass> {
        self.ssc_features().next()
    }
}

impl IntoIterator for Discovery {
    type Item = FeatureDescriptor;
    type IntoIter = <Vec<FeatureDescriptor> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.feature_descriptors.into_iter()
    }
}

impl FromIterator<FeatureDescriptor> for Discovery {
    fn from_iter<T: IntoIterator<Item = FeatureDescriptor>>(iter: T) -> Self {
        Self::from(iter.into_iter().collect::<Vec<_>>())
    }
}

impl From<Vec<FeatureDescriptor>> for Discovery {
    fn from(value: Vec<FeatureDescriptor>) -> Self {
        Self { feature_descriptors: value, ..Default::default() }
    }
}

impl Default for Discovery {
    fn default() -> Self {
        Self {
            major_version: 0x0000,
            minor_version: 0x0001,
            vendor_unique: Default::default(),
            feature_descriptors: Default::default(),
        }
    }
}

impl MultiPassSerialize for Discovery {
    fn serialize<S: RevisableSerializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
        serializer.with_byte_order(ByteOrder::BigEndian, |se| {
            let (span, length_span) = se.serialize_composite(|se| {
                let length: u32 = 0;
                let length_span = length.serialize(se)?;
                self.major_version.serialize(se)?;
                self.minor_version.serialize(se)?;
                se.pad(16)?;
                self.vendor_unique.serialize(se)?;
                collection::items(&self.feature_descriptors).serialize(se)?;
                Ok(length_span)
            })?;
            se.revise_span(&length_span, |se| {
                let length: u32 = span.len() as u32 - 4;
                length.serialize(se)
            })?;
            Ok(span)
        })
    }
}

impl Deserialize for Discovery {
    fn deserialize<D: sorbit::ser_de::Deserializer>(deserializer: &mut D) -> Result<Self, D::Error> {
        deserializer.with_byte_order(ByteOrder::BigEndian, |deserializer| {
            let length = u32::deserialize(deserializer)?;
            let major_version = u16::deserialize(deserializer)?;
            let minor_version = u16::deserialize(deserializer)?;
            deserializer.pad(16)?;
            let vendor_unique = <[u8; 32]>::deserialize(deserializer)?;
            let byte_count = length - 44;
            let feature_descriptors = collection::deserialize_items_by_byte_count(deserializer, &byte_count)?;
            let mut discovery = Self { major_version, minor_version, vendor_unique, feature_descriptors };
            discovery.feature_descriptors.retain(|desc| !matches!(desc, FeatureDescriptor::Unrecognized(0, _)));
            Ok(discovery)
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

#[cfg(test)]
mod tests {
    use sorbit::ser_de::{FromBytes, ToBytes};

    use super::*;

    #[rustfmt::skip]
    const TPER_DESC_BYTES : [u8; 16] = [
        0x00, 0x01, // Feature code.
        0x10, // Version | reserved.
        0x0C, // Length.
        0b0101_0101, // Flags,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // Reserved.
    ];

    const TPER_DESC_VALUE: FeatureDescriptor = FeatureDescriptor::TPer(TperDescriptor {
        version: PhantomData,
        length: PhantomData,
        com_id_mgmt_supported: true,
        streaming_supported: true,
        buffer_mgmt_supported: false,
        ack_nak_supported: true,
        async_supported: false,
        sync_supported: true,
    });

    #[test]
    fn serialize_tper_desc() {
        let bytes = TPER_DESC_BYTES;
        let value = TPER_DESC_VALUE;
        assert_eq!(value.to_bytes().unwrap(), &bytes);
        assert_eq!(FeatureDescriptor::from_bytes(&bytes).unwrap(), value);
    }

    #[rustfmt::skip]
    const LOCKING_DESC_BYTES : [u8; 16] = [
        0x00, 0x02, // Feature code.
        0x10, // Version | reserved.
        0x0C, // Length.
        0b0101_0101, // Flags,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // Reserved.
    ];
    const LOCKING_DESC_VALUE: FeatureDescriptor = FeatureDescriptor::Locking(LockingDescriptor {
        version: PhantomData,
        length: PhantomData,
        hw_reset_supported: false,
        mbr_shadowing_not_supported: true,
        mbr_done: false,
        mbr_enabled: true,
        media_encryption: false,
        locked: true,
        locking_enabled: false,
        locking_supported: true,
    });

    #[test]
    fn serialize_locking_desc() {
        #[rustfmt::skip]
        let bytes = LOCKING_DESC_BYTES;
        let value = LOCKING_DESC_VALUE;
        assert_eq!(value.to_bytes().unwrap(), &bytes);
        assert_eq!(FeatureDescriptor::from_bytes(&bytes).unwrap(), value);
    }

    #[test]
    fn serialize_geometry_desc() {
        #[rustfmt::skip]
        let bytes = [
            0x00, 0x03, // Feature code.
            0x10, // Version | reserved.
            0x1C, // Length.
            1, // Align.
            0, 0, 0, 0, 0, 0, 0, // Reserved.
            0x00, 0x00, 0x00, 0x50, // Logical block size.
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x60, // Alignment.
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x70, // Lowest LBA.
        ];
        let value = FeatureDescriptor::Geometry(GeometryDescriptor {
            version: PhantomData,
            length: PhantomData,
            align: true,
            logical_block_size: 0x50,
            alignment_granularity: 0x60,
            lowest_aligned_lba: 0x70,
        });
        assert_eq!(value.to_bytes().unwrap(), &bytes);
        assert_eq!(FeatureDescriptor::from_bytes(&bytes).unwrap(), value);
    }

    #[test]
    fn serialize_data_removal_desc() {
        #[rustfmt::skip]
        let bytes = [
            0x04, 0x04, // Feature code.
            0x20, // Version | reserved.
            0x20, // Length.
            0, // Reserved.
            0b10, // Interrupted / processing.
            0b0010_0101, // Supported mechanism.
            0b0010_0101, // Format.
            0x00, 0x01, // Vendor erase time
            0x00, 0x02, // Crypto erase time
            0x00, 0x03, // Block erase time
            0, 0, 0, 0, // Reserved
            0x00, 0x04, // Overwrite time
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // Reserved

        ];
        let value = FeatureDescriptor::DataRemoval(DataRemovalDescriptor {
            version: PhantomData,
            length: PhantomData,
            interrupted: true,
            processing: false,
            supported_mechanism: DataRemovalMechanism {
                vendor_erase: true,
                crypto_erase: true,
                block_erase: false,
                overwrite: true,
            },
            removal_time: DataRemovalTime {
                vendor_erase_unit: true,
                crypto_erase_unit: true,
                block_erase_unit: false,
                overwrite_unit: true,
                overwrite_amount: 1,
                block_erase_amount: 2,
                crypto_erase_amount: 3,
                vendor_erase_amount: 4,
            },
        });
        assert_eq!(value.to_bytes().unwrap(), &bytes);
        assert_eq!(FeatureDescriptor::from_bytes(&bytes).unwrap(), value);
    }

    #[test]
    fn serialize_block_sid_desc() {
        #[rustfmt::skip]
        let bytes = [
            0x04, 0x02, // Feature code.
            0x20, // Version | reserved.
            0x0C, // Length.
            0b0000_0101, // Flags.
            0b0000_0001, // HW reset.
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // Reserved.
        ];
        let value = FeatureDescriptor::BlockSIDAuth(BlockSIDAuthDescriptor {
            version: 0x02,
            length: PhantomData,
            locking_sp_frozen: false,
            locking_sp_freeze_supported: true,
            sid_authentication_blocked: false,
            sid_msid_pin_differ: true,
            hw_reset_unblocks: true,
        });
        assert_eq!(value.to_bytes().unwrap(), &bytes);
        assert_eq!(FeatureDescriptor::from_bytes(&bytes).unwrap(), value);
    }

    #[test]
    fn serialize_additional_data_store_desc() {
        #[rustfmt::skip]
        let bytes = [
            0x02, 0x02, // Feature code.
            0x21, // Version | minor version.
            0x0C, // Length.
            0, 0, // Reserved
            0x00, 0x01, // Max num tables
            0x00, 0x00, 0x00, 0x02, // Max total size
            0x00, 0x00, 0x00, 0x03, // Alignment

        ];
        let value = FeatureDescriptor::AdditionalDataStoreTables(AdditionalDataStoreTablesDescriptor {
            version: PhantomData,
            length: PhantomData,
            minor_version: 1,
            max_num_tables: 0x01,
            max_total_size_of_tables: 0x02,
            table_size_alignment: 0x03,
        });
        assert_eq!(value.to_bytes().unwrap(), &bytes);
        assert_eq!(FeatureDescriptor::from_bytes(&bytes).unwrap(), value);
    }

    #[test]
    fn serialize_enterprise_desc() {
        #[rustfmt::skip]
        let bytes = [
            0x01, 0x00, // Feature code.
            0b00001_000, // Version.
            0x10, // Length.
            0x00, 0x02, // Base ComID
            0x00, 0x01, // Num ComIDs
            0b0000_0001, // Range crossing
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // Reserved
        ];
        let value = FeatureDescriptor::Enterprise(EnterpriseDescriptor {
            version: 1,
            length: PhantomData,
            base_com_id: 2,
            num_com_ids: 1,
            no_range_crossing: true,
        });
        assert_eq!(value.to_bytes().unwrap(), &bytes);
        assert_eq!(FeatureDescriptor::from_bytes(&bytes).unwrap(), value);
    }

    #[test]
    fn serialize_opal_v1_desc() {
        #[rustfmt::skip]
        let bytes = [
            0x02, 0x00, // Feature code.
            0b0001_0000, // Version.
            0x10, // Length.
            0x00, 0x02, // Base ComID
            0x00, 0x01, // Num ComIDs
            0b0000_0001, // Range crossing
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // Reserved
        ];
        let value = FeatureDescriptor::OpalV1(OpalV1Descriptor {
            version: 1,
            length: PhantomData,
            base_com_id: 2,
            num_com_ids: 1,
            no_range_crossing: true,
        });
        assert_eq!(value.to_bytes().unwrap(), &bytes);
        assert_eq!(FeatureDescriptor::from_bytes(&bytes).unwrap(), value);
    }

    #[test]
    fn serialize_opal_v2_desc() {
        #[rustfmt::skip]
        let bytes = [
            0x02, 0x03, // Feature code.
            0b0001_0000, // Version.
            0x10, // Length.
            0x00, 0x02, // Base ComID
            0x00, 0x01, // Num ComIDs
            0b0000_0001, // Range crossing
            0x00, 0x04, // Num admins
            0x00, 0x08, // Num users
            0xFF, // Initial PIN
            0xFF, // Reverted PIN
            0, 0, 0, 0, 0, // Reserved
        ];
        let value = FeatureDescriptor::OpalV2(OpalV2Descriptor {
            version: 1,
            length: PhantomData,
            base_com_id: 2,
            num_com_ids: 1,
            no_range_crossing: true,
            num_locking_admins_supported: 4,
            num_locking_users_supported: 8,
            initial_owner_pw: OwnerPasswordState::VendorSpecified,
            reverted_owner_pw: OwnerPasswordState::VendorSpecified,
        });
        assert_eq!(value.to_bytes().unwrap(), &bytes);
        assert_eq!(FeatureDescriptor::from_bytes(&bytes).unwrap(), value);
    }

    #[test]
    fn serialize_opalite_desc() {
        #[rustfmt::skip]
        let bytes = [
            0x03, 0x01, // Feature code.
            0b0001_0000, // Version.
            0x10, // Length.
            0x00, 0x02, // Base ComID
            0x00, 0x01, // Num ComIDs
            0, 0, 0, 0, 0, // Reserved
            0xFF, // Initial PIN
            0xFF, // Reverted PIN
            0, 0, 0, 0, 0, // Reserved
        ];
        let value = FeatureDescriptor::Opalite(OpaliteDescriptor {
            version: 1,
            length: PhantomData,
            base_com_id: 2,
            num_com_ids: 1,
            initial_owner_pw: OwnerPasswordState::VendorSpecified,
            reverted_owner_pw: OwnerPasswordState::VendorSpecified,
        });
        assert_eq!(value.to_bytes().unwrap(), &bytes);
        assert_eq!(FeatureDescriptor::from_bytes(&bytes).unwrap(), value);
    }

    #[test]
    fn serialize_pyrite_v1_desc() {
        #[rustfmt::skip]
        let bytes = [
            0x03, 0x02, // Feature code.
            0b0001_0000, // Version.
            0x10, // Length.
            0x00, 0x02, // Base ComID
            0x00, 0x01, // Num ComIDs
            0, 0, 0, 0, 0, // Reserved
            0xFF, // Initial PIN
            0xFF, // Reverted PIN
            0, 0, 0, 0, 0, // Reserved
        ];
        let value = FeatureDescriptor::PyriteV1(PyriteV1Descriptor {
            version: 1,
            length: PhantomData,
            base_com_id: 2,
            num_com_ids: 1,
            initial_owner_pw: OwnerPasswordState::VendorSpecified,
            reverted_owner_pw: OwnerPasswordState::VendorSpecified,
        });
        assert_eq!(value.to_bytes().unwrap(), &bytes);
        assert_eq!(FeatureDescriptor::from_bytes(&bytes).unwrap(), value);
    }

    #[test]
    fn serialize_pyrite_v2_desc() {
        #[rustfmt::skip]
        let bytes = [
            0x03, 0x03, // Feature code.
            0b0001_0000, // Version.
            0x10, // Length.
            0x00, 0x02, // Base ComID
            0x00, 0x01, // Num ComIDs
            0, 0, 0, 0, 0, // Reserved
            0xFF, // Initial PIN
            0xFF, // Reverted PIN
            0, 0, 0, 0, 0, // Reserved
        ];
        let value = FeatureDescriptor::PyriteV2(PyriteV2Descriptor {
            version: 1,
            length: PhantomData,
            base_com_id: 2,
            num_com_ids: 1,
            initial_owner_pw: OwnerPasswordState::VendorSpecified,
            reverted_owner_pw: OwnerPasswordState::VendorSpecified,
        });
        assert_eq!(value.to_bytes().unwrap(), &bytes);
        assert_eq!(FeatureDescriptor::from_bytes(&bytes).unwrap(), value);
    }

    #[test]
    fn serialize_ruby_desc() {
        #[rustfmt::skip]
        let bytes = [
            0x03, 0x04, // Feature code.
            0b0001_0000, // Version.
            0x10, // Length.
            0x00, 0x02, // Base ComID
            0x00, 0x01, // Num ComIDs
            0b0000_0001, // Range crossing
            0x00, 0x04, // Num admins
            0x00, 0x08, // Num users
            0xFF, // Initial PIN
            0xFF, // Reverted PIN
            0, 0, 0, 0, 0, // Reserved
        ];
        let value = FeatureDescriptor::Ruby(RubyDescriptor {
            version: 1,
            length: PhantomData,
            base_com_id: 2,
            num_com_ids: 1,
            no_range_crossing: true,
            num_locking_admins_supported: 4,
            num_locking_users_supported: 8,
            initial_owner_pw: OwnerPasswordState::VendorSpecified,
            reverted_owner_pw: OwnerPasswordState::VendorSpecified,
        });
        assert_eq!(value.to_bytes().unwrap(), &bytes);
        assert_eq!(FeatureDescriptor::from_bytes(&bytes).unwrap(), value);
    }

    #[test]
    fn serialize_kpio_desc() {
        #[rustfmt::skip]
        let bytes = [
            0x03, 0x05, // Feature code.
            0b0001_0000, // Version.
            0x10, // Length.
            0x00, 0x02, // Base ComID 0x01
            0x00, 0x01, // Num ComIDs 0x01
            0x00, 0x04, // Base ComID 0x03
            0x00, 0x01, // Num ComIDs 0x03
            0xFF, // Initial PIN
            0xFF, // Reverted PIN
            0x00, 0x04, // Num admins
            0b0001_0101, // Flags @ 16
            0x01, 0x00, // Max unique identifier len
            0b0000_0001, // KMIP
            0, // Reserved
            0b0000_0101, // Flags @ 21
            0, // Reserved
            0b0000_0001, // AES256 wrapping key
            0, // Reserved
            0b0000_0101, // RSA 2/3/4 wrapping keys
            0, // Reserved
            0b0000_0011, // PKI KEK
            0, 0, 0, 0, // Reserved
            0x00, 0x00, 0x00, 0x01, // Num KEKs
            0x00, 0x00, 0x00, 0x02, // Num key tags
            0x00, 0x03, // Key tags per NS
            0x04, // None length
            0, 0, 0, 0, 0, // Reserved
        ];
        let value = FeatureDescriptor::KeyPerIO(KeyPerIODescriptor {
            version: 1,
            minor_version: 0,
            length: PhantomData,
            base_com_id_p1: 2,
            num_com_ids_p1: 1,
            base_com_id_p3: 4,
            num_com_ids_p3: 1,
            initial_owner_pw: OwnerPasswordState::VendorSpecified,
            reverted_owner_pw: OwnerPasswordState::VendorSpecified,
            num_kpio_admins_supported: 4,
            replay_protection_enabled: false,
            replay_protection_supported: true,
            incorrect_key_detection_supported: false,
            tweak_key_required: true,
            kpio_scope: false,
            kpio_enabled: true,
            max_key_uid_len: 256,
            kmip_key_injection_supported: true,
            nist_rsa_oaep_supported: true,
            nist_aes_gcm_supported: false,
            nist_aes_kw_supported: true,
            aes256_wrapping_supported: true,
            rsa4k_wrapping_supported: true,
            rsa3k_wrapping_supported: false,
            rsa2k_wrapping_supported: true,
            pki_kek_transport_supported: true,
            plaintext_kek_prov_supported: true,
            num_keks_supported: 1,
            total_key_tags_supported: 2,
            max_key_tags_per_namespace: 3,
            get_nonce_cmd_nonce_len: 4,
        });
        assert_eq!(value.to_bytes().unwrap(), &bytes);
        assert_eq!(FeatureDescriptor::from_bytes(&bytes).unwrap(), value);
    }

    #[test]
    pub fn serialize_discovery() {
        #[rustfmt::skip]
        let header = [
            0, 0, 0, 76, // Length
            0, 0, // Major version
            0, 1, // Minor version
        ];
        let reserved_and_vu = [0u8; 40];
        let bytes: Vec<_> =
            header.into_iter().chain(reserved_and_vu).chain(TPER_DESC_BYTES).chain(LOCKING_DESC_BYTES).collect();
        let value = Discovery {
            major_version: 0,
            minor_version: 1,
            vendor_unique: Default::default(),
            feature_descriptors: vec![TPER_DESC_VALUE, LOCKING_DESC_VALUE],
        };

        assert_eq!(value.to_bytes().unwrap(), bytes);
        assert_eq!(Discovery::from_bytes(&bytes).unwrap(), value);
    }
}
