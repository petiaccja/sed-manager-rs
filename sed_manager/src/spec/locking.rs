//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_spec::objects::SecurityProviderRef;

#[derive(Debug)]
pub struct Locking {
    pub uid: SecurityProviderRef,
}
