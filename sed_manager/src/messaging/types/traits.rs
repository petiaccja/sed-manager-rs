use crate::messaging::uid::UID;
use crate::messaging::value::Value;

pub trait Type: Into<Value> + TryFrom<Value> {
    fn uid() -> UID;
    fn name() -> &'static str;
}

macro_rules! declare_type {
    ($type:ty, $uid:expr, $name:expr) => {
        impl crate::messaging::types::traits::Type for $type {
            fn uid() -> crate::messaging::uid::UID {
                $uid.into()
            }
            fn name() -> &'static str {
                $name.into()
            }
        }
    };
}

pub(crate) use declare_type;
