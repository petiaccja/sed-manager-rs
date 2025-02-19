use sed_manager_macros::AliasType;

use crate::{messaging::value::Bytes, specification::basic_types::MaxBytes};

use super::define_column_type;

define_column_type!(Bytes, 0x0000_0005_0000_0002_u64, "bytes");
define_column_type!(Bytes4, 0x0000_0005_0000_0238_u64, "bytes_4");
define_column_type!(Bytes12, 0x0000_0005_0000_0201_u64, "bytes_12");
define_column_type!(Bytes16, 0x0000_0005_0000_0202_u64, "bytes_16");
define_column_type!(Bytes20, 0x0000_0005_0000_0236_u64, "bytes_20");
define_column_type!(Bytes32, 0x0000_0005_0000_0205_u64, "bytes_32");
define_column_type!(Bytes48, 0x0000_0005_0000_0237_u64, "bytes_48");
define_column_type!(Bytes64, 0x0000_0005_0000_0206_u64, "bytes_64");
define_column_type!(MaxBytes32, 0x0000_0005_0000_020D_u64, "max_bytes_32");
define_column_type!(MaxBytes64, 0x0000_0005_0000_020D_u64, "max_bytes_64");
define_column_type!(Name, 0x0000_0005_0000_020B_u64, "name");

pub type Bytes4 = [u8; 4];
pub type Bytes12 = [u8; 12];
pub type Bytes16 = [u8; 16];
pub type Bytes20 = [u8; 20];
pub type Bytes32 = [u8; 32];
pub type Bytes48 = [u8; 48];
pub type Bytes64 = [u8; 64];
pub type MaxBytes32 = MaxBytes<32>;
pub type MaxBytes64 = MaxBytes<64>;

#[derive(AliasType, PartialEq, Eq, Clone, Debug, Default)]
pub struct Name(MaxBytes32);

#[derive(AliasType, PartialEq, Eq, Clone, Debug, Default)]
pub struct Password(MaxBytes32);

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
