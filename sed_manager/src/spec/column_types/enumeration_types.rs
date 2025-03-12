use sed_manager_macros::{AliasType, EnumerationType};

use super::define_column_type;

define_column_type!(MessagingType, 0x0000_0005_0000_0404_u64, "messaging_type");
define_column_type!(HashProtocol, 0x0000_0005_0000_040D_u64, "hash_protocol");
define_column_type!(AuthMethod, 0x0000_0005_0000_0408_u64, "auth_method");
define_column_type!(Day, 0x0000_0005_0000_0418_u64, "day_enum");
define_column_type!(Month, 0x0000_0005_0000_0417_u64, "month_enum");
define_column_type!(Year, 0x0000_0005_0000_0416_u64, "year_enum");
define_column_type!(LogSelect, 0x0000_0005_0000_040C_u64, "log_select");

#[repr(u8)]
#[derive(EnumerationType, PartialEq, Eq, Clone, Debug)]
pub enum LifeCycleState {
    Issued = 0,
    IssuedDisabled = 1,
    IssuedFrozen = 2,
    IssuedDisabledFrozen = 3,
    IssuedFailed = 4,
    ManufacturedInactive = 8,
    Manufactured = 9,
    ManufacturedDisabled = 10,
    ManufacturedFrozen = 11,
    ManufacturedDisabledFrozen = 12,
    ManufacturedFailed = 13,
    #[fallback]
    Unknown = 255,
}

#[repr(u8)]
#[derive(EnumerationType, PartialEq, Eq, Clone, Debug)]
pub enum AuthMethod {
    None = 0,
    Password = 1,
    Exchange = 2,
    Sign = 3,
    SymK = 4,
    HMAC = 5,
    TPerSign = 6,
    TPerExchange = 7,
    #[fallback]
    Unknown = 255,
}

#[repr(u8)]
#[derive(EnumerationType, PartialEq, Eq, Clone, Debug)]
pub enum LogSelect {
    None = 0,
    LogSuccess = 1,
    LogFail = 2,
    LogAlways = 3,
}

#[repr(u8)]
#[derive(EnumerationType, PartialEq, Eq, Clone, Debug)]
pub enum HashProtocol {
    None = 0,
    SHA1 = 1,
    SHA256 = 2,
    SHA384 = 3,
    SHA512 = 4,
    #[fallback]
    Unknown = 255,
}

#[allow(non_camel_case_types)]
#[repr(u8)]
#[derive(EnumerationType, PartialEq, Eq, Clone, Debug)]
pub enum MessagingType {
    None = 0,
    HMAC_SHA_256 = 1,
    HMAC_SHA_384 = 2,
    HMAC_SHA_512 = 3,
    RSASSA_PKCS1_v1_5_1024 = 4,
    RSASSA_PKCS1_v1_5_2048 = 5,
    RSASSA_PKCS1_v1_5_3072 = 6,
    RSASSA_PSS_1024 = 7,
    RSASSA_PSS_2048 = 8,
    RSASSA_PSS_3072 = 9,
    ECDSA_256_SHA_256 = 10,
    ECDSA_384_SHA_384 = 11,
    ECDSA_512_SHA_512 = 12,
    CMAC_128_with_128_bit_MAC = 13,
    CMAC_256_with_128_bit_MAC = 14,
    GMAC_128_with_128_bit_MAC_and_96_bit_IV = 15,
    GMAC_256_with_128_bit_MAC_and_96_bit_IV = 16,
    AES_CBC_128 = 64,
    AES_CBC_256 = 65,
    AES_CBC_128_with_HMAC_SHA_256 = 129,
    AES_CBC_256_with_HMAC_SHA_256 = 130,
    AES_CBC_256_with_HMAC_SHA_384 = 131,
    AES_CBC_256_with_HMAC_SHA_512 = 132,
    AES_CCM_128_with_128_bit_MAC = 133,
    AES_CCM_256_with_128_bit_MAC = 134,
    AES_GCM_128_with_128_bit_MAC = 135,
    AES_GCM_256_with_128_bit_MAC = 136,
    #[fallback]
    Unknown = 255,
}

#[derive(AliasType, PartialEq, Eq, Clone, Debug, Default)]
pub struct Year(u16);

#[derive(AliasType, PartialEq, Eq, Clone, Debug, Default)]
pub struct Month(u8);

#[derive(AliasType, PartialEq, Eq, Clone, Debug, Default)]
pub struct Day(u8);

#[repr(u8)]
#[derive(EnumerationType, PartialEq, Eq, Clone, Debug)]
pub enum ReencryptState {
    Idle = 1,
    Pending = 2,
    Active = 3,
    Completed = 4,
    Paused = 5,
    #[fallback]
    Unknown = 16,
}

#[repr(u8)]
#[derive(EnumerationType, PartialEq, Eq, Clone, Debug)]
pub enum ReencryptRequest {
    // This is not empty as a mistake.
    // The values are missing from the core specification.
    #[fallback]
    Unknown = 16,
}

#[repr(u8)]
#[derive(EnumerationType, PartialEq, Eq, Clone, Debug)]
pub enum AdvKeyMode {
    WaitForAdvKeyReq = 0,
    AutoAdvanceKeys = 1,
    #[fallback]
    Unknown = 7,
}

#[repr(u8)]
#[derive(EnumerationType, PartialEq, Eq, Clone, Debug)]
pub enum VerifyMode {
    NoVerify = 0,
    VerifyEnabled = 1,
    #[fallback]
    Unknown = 7,
}

#[repr(u8)]
#[derive(EnumerationType, PartialEq, Eq, Clone, Debug)]
pub enum LastReencStatus {
    Success = 0,
    ReadError = 1,
    WriteError = 2,
    VerifyError = 3,
    #[fallback]
    Unknown = 7,
}

#[repr(u8)]
#[derive(EnumerationType, PartialEq, Eq, Clone, Debug)]
pub enum GeneralStatus {
    None = 0,
    PendingTPerError = 1,
    ActiveTPerError = 2,
    ActivePauseRequest = 3,
    PendingPauseRequested = 4,
    PendingResetStopDetected = 5,
    KeyError = 6,
    WaitAvailableKeys = 32,
    WaitForTPerResources = 33,
    ActiveResetStopDetected = 34,
    #[fallback]
    Unknown = 63,
}

#[repr(u8)]
#[derive(EnumerationType, PartialEq, Eq, Clone, Debug)]
pub enum SymmetricModeMedia {
    ECB = 0,
    CBC = 1,
    CFB = 2,
    OFB = 3,
    GCM = 4,
    CTR = 5,
    CCM = 6,
    XTS = 7,
    LRW = 8,
    EME = 9,
    CMC = 10,
    XEX = 11,
    MediaEncryption = 23,
    #[fallback]
    Unknown = 22,
}

#[repr(u8)]
#[derive(EnumerationType, PartialEq, Eq, Clone, Debug)]
pub enum TableKind {
    Object = 1,
    Byte = 2,
    #[fallback]
    Unknown = 8,
}

#[repr(u8)]
#[derive(EnumerationType, PartialEq, Eq, Clone, Debug, Copy)]
pub enum BooleanOp {
    And = 0,
    Or = 1,
    Not = 2,
}
