//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_spec::objects::SecurityProviderRef;

#[derive(Debug, Clone)]
pub struct Locking {
    pub uid: SecurityProviderRef,
}
