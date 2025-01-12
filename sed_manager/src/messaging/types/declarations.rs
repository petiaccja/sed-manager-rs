use super::primitives::MaxBytes;
use super::traits::declare_type;

pub type Bytes4 = [u8; 4];
pub type Bytes12 = [u8; 12];
pub type Bytes16 = [u8; 16];
pub type Bytes20 = [u8; 20];
pub type Bytes32 = [u8; 32];
pub type Bytes48 = [u8; 48];
pub type Bytes64 = [u8; 64];
pub type MaxBytes32 = MaxBytes<32>;
pub type MaxBytes64 = MaxBytes<64>;

declare_type!(i8, 0x0000_0005_0000_0210_u64, "integer_1");
declare_type!(i16, 0x0000_0005_0000_0215_u64, "integer_2");
declare_type!(u8, 0x0000_0005_0000_0211_u64, "uinteger_1");
declare_type!(u16, 0x0000_0005_0000_0216_u64, "uinteger_2");
declare_type!(u32, 0x0000_0005_0000_0220_u64, "uinteger_4");
declare_type!(u64, 0x0000_0005_0000_0225_u64, "uinteger_8");
declare_type!(Bytes4, 0x0000_0005_0000_0238_u64, "bytes_4");
declare_type!(Bytes12, 0x0000_0005_0000_0201_u64, "bytes_12");
declare_type!(Bytes16, 0x0000_0005_0000_0202_u64, "bytes_16");
declare_type!(Bytes20, 0x0000_0005_0000_0236_u64, "bytes_20");
declare_type!(Bytes32, 0x0000_0005_0000_0205_u64, "bytes_32");
declare_type!(Bytes48, 0x0000_0005_0000_0237_u64, "bytes_48");
declare_type!(Bytes64, 0x0000_0005_0000_0206_u64, "bytes_64");
declare_type!(MaxBytes32, 0x0000_0005_0000_020D_u64, "max_bytes_32");
declare_type!(MaxBytes64, 0x0000_0005_0000_020D_u64, "max_bytes_64");
