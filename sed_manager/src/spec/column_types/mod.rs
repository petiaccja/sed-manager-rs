mod alternative_types;
mod byte_types;
mod enumeration_types;
mod method_types;
mod reference_types;
mod set_types;
mod struct_types;

macro_rules! define_column_type {
    ($type:ty, $uid:expr, $name:expr) => {
        impl<'me> crate::spec::basic_types::Type for $type {
            fn uid() -> crate::messaging::uid::UID {
                $uid.into()
            }
            fn name() -> &'static str {
                $name.into()
            }
        }
    };
}

pub(crate) use define_column_type;

use crate::messaging::uid::UID;

pub use alternative_types::*;
pub use byte_types::*;
pub use enumeration_types::*;
pub use method_types::*;
pub use reference_types::*;
pub use set_types::*;
pub use struct_types::*;

define_column_type!(UID, 0x0000_0005_0000_0209_u64, "uid");
define_column_type!(bool, 0x0000_0005_0000_0401_u64, "boolean");
define_column_type!(i8, 0x0000_0005_0000_0210_u64, "integer_1");
define_column_type!(i16, 0x0000_0005_0000_0215_u64, "integer_2");
define_column_type!(u8, 0x0000_0005_0000_0211_u64, "uinteger_1");
define_column_type!(u16, 0x0000_0005_0000_0216_u64, "uinteger_2");
define_column_type!(u32, 0x0000_0005_0000_0220_u64, "uinteger_4");
define_column_type!(u64, 0x0000_0005_0000_0225_u64, "uinteger_8");
define_column_type!(BooleanOp, 0x0000_0005_0000_040E_u64, "boolean_ACE");
