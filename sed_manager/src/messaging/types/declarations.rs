use sed_manager_macros::{AliasType, EnumerationType, StructType};

use crate::messaging::uid::UID;
pub use crate::messaging::value::Bytes;
use crate::messaging::value::{List, Value};
use crate::specification::tables;

use super::max_bytes::MaxBytes;
use super::traits::declare_type;
use super::RestrictedObjectReference;

pub type Bytes4 = [u8; 4];
pub type Bytes12 = [u8; 12];
pub type Bytes16 = [u8; 16];
pub type Bytes20 = [u8; 20];
pub type Bytes32 = [u8; 32];
pub type Bytes48 = [u8; 48];
pub type Bytes64 = [u8; 64];
pub type MaxBytes32 = MaxBytes<32>;
pub type MaxBytes64 = MaxBytes<64>;

pub type AuthorityRef = RestrictedObjectReference<{ tables::AUTHORITY.value() }>;
pub type SPRef = RestrictedObjectReference<{ tables::SP.value() }>;
pub type CPinRef = RestrictedObjectReference<{ tables::C_PIN.value() }>;
pub type CredentialRef = CPinRef; // Should have more tables but it's difficult to express without variadics.
pub type LogListRef = RestrictedObjectReference<{ tables::LOG_LIST.value() }>;

/// Result returned by the Authenticate method.
/// I'm guessing it's not encoded as an NVP like regular typeOr{} objects, but simply as plain data.
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum BoolOrBytes {
    Bool(bool),
    Bytes(Bytes),
}

/// Represents the result of the Get method.
/// According to the TCG examples, it's not encoded as an NVP like regular typeOr{} objects, but simply as plain data.
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum BytesOrRowValues {
    Bytes(Bytes),
    RowValues(List),
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

#[derive(AliasType, PartialEq, Eq, Clone, Debug, Default)]
pub struct Name(MaxBytes32);

#[derive(AliasType, PartialEq, Eq, Clone, Debug, Default)]
pub struct Password(MaxBytes32);

#[derive(AliasType, PartialEq, Eq, Clone, Debug, Default)]
pub struct Year(u16);

#[derive(AliasType, PartialEq, Eq, Clone, Debug, Default)]
pub struct Month(u8);

#[derive(AliasType, PartialEq, Eq, Clone, Debug, Default)]
pub struct Day(u8);

#[derive(StructType, PartialEq, Eq, Clone, Debug, Default)]
pub struct Date {
    year: Year,
    month: Month,
    day: Day,
}

declare_type!(UID, 0x0000_0005_0000_0209_u64, "uid");
declare_type!(bool, 0x0000_0005_0000_0401_u64, "boolean");
declare_type!(i8, 0x0000_0005_0000_0210_u64, "integer_1");
declare_type!(i16, 0x0000_0005_0000_0215_u64, "integer_2");
declare_type!(u8, 0x0000_0005_0000_0211_u64, "uinteger_1");
declare_type!(u16, 0x0000_0005_0000_0216_u64, "uinteger_2");
declare_type!(u32, 0x0000_0005_0000_0220_u64, "uinteger_4");
declare_type!(u64, 0x0000_0005_0000_0225_u64, "uinteger_8");
declare_type!(Bytes, 0x0000_0005_0000_0002_u64, "bytes");
declare_type!(Bytes4, 0x0000_0005_0000_0238_u64, "bytes_4");
declare_type!(Bytes12, 0x0000_0005_0000_0201_u64, "bytes_12");
declare_type!(Bytes16, 0x0000_0005_0000_0202_u64, "bytes_16");
declare_type!(Bytes20, 0x0000_0005_0000_0236_u64, "bytes_20");
declare_type!(Bytes32, 0x0000_0005_0000_0205_u64, "bytes_32");
declare_type!(Bytes48, 0x0000_0005_0000_0237_u64, "bytes_48");
declare_type!(Bytes64, 0x0000_0005_0000_0206_u64, "bytes_64");
declare_type!(MaxBytes32, 0x0000_0005_0000_020D_u64, "max_bytes_32");
declare_type!(MaxBytes64, 0x0000_0005_0000_020D_u64, "max_bytes_64");

declare_type!(AuthorityRef, 0x0000_0005_0000_0C05_u64, "Authority_object_ref");
declare_type!(CredentialRef, 0x0000_0005_0000_0C0B_u64, "cred_object_uidref");
declare_type!(LogListRef, 0x0000_0005_0000_0C0D_u64, "LogList_object_ref");

declare_type!(Name, 0x0000_0005_0000_020B_u64, "name");
declare_type!(MessagingType, 0x0000_0005_0000_0404_u64, "messaging_type");
declare_type!(HashProtocol, 0x0000_0005_0000_040D_u64, "hash_protocol");
declare_type!(AuthMethod, 0x0000_0005_0000_0408_u64, "auth_method");
declare_type!(Day, 0x0000_0005_0000_0418_u64, "day_enum");
declare_type!(Month, 0x0000_0005_0000_0417_u64, "month_enum");
declare_type!(Year, 0x0000_0005_0000_0416_u64, "year_enum");
declare_type!(Date, 0x0000_0005_0000_1804_u64, "date");
declare_type!(LogSelect, 0x0000_0005_0000_040C_u64, "log_select");

impl From<&str> for Name {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl From<&Name> for &str {
    fn from(value: &Name) -> Self {
        value.into()
    }
}

impl From<String> for Name {
    fn from(value: String) -> Self {
        Self(value.into())
    }
}

impl From<&Name> for String {
    fn from(value: &Name) -> Self {
        value.into()
    }
}

impl From<&str> for Password {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl From<&Password> for &str {
    fn from(value: &Password) -> Self {
        value.into()
    }
}

impl From<String> for Password {
    fn from(value: String) -> Self {
        Self(value.into())
    }
}

impl From<&Password> for String {
    fn from(value: &Password) -> Self {
        value.into()
    }
}

impl TryFrom<Value> for BoolOrBytes {
    type Error = Value;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let maybe_bool = bool::try_from(value).map(|x| BoolOrBytes::Bool(x));
        let value = match maybe_bool {
            Ok(x) => return Ok(x),
            Err(v) => v,
        };
        Bytes::try_from(value).map(|x| BoolOrBytes::Bytes(x))
    }
}

impl From<BoolOrBytes> for Value {
    fn from(value: BoolOrBytes) -> Self {
        match value {
            BoolOrBytes::Bool(x) => x.into(),
            BoolOrBytes::Bytes(x) => x.into(),
        }
    }
}

impl TryFrom<Value> for BytesOrRowValues {
    type Error = Value;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let maybe_bool = Bytes::try_from(value).map(|x| BytesOrRowValues::Bytes(x));
        let value = match maybe_bool {
            Ok(x) => return Ok(x),
            Err(v) => v,
        };
        List::try_from(value).map(|x| BytesOrRowValues::RowValues(x))
    }
}

impl From<BytesOrRowValues> for Value {
    fn from(value: BytesOrRowValues) -> Self {
        match value {
            BytesOrRowValues::Bytes(x) => x.into(),
            BytesOrRowValues::RowValues(x) => x.into(),
        }
    }
}
