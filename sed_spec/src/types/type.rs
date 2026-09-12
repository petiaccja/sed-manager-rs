//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::objects::TypeRef;

pub trait Type {
    const UID: TypeRef;
}
