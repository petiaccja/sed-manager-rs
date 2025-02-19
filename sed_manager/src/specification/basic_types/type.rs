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
