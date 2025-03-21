//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

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
