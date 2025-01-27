use crate::messaging::uid::UID;
use crate::messaging::value::Value;

pub trait Type
where
    Value: From<Self>,
    Self: TryFrom<Value>,
{
    fn uid() -> UID;
    fn name() -> &'static str;
}

macro_rules! declare_type {
    ($type:ty, $uid:expr, $name:expr) => {
        impl<'me> crate::messaging::types::traits::Type for $type {
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
