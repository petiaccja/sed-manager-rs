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
