//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_spec::objects::{AuthorityRef, CPinRef, SecurityProviderRef};

#[derive(Debug, Clone)]
pub struct Admin {
    pub uid: SecurityProviderRef,
    pub authorities: AuthorityTable,
    pub c_pins: CPinTable,
}

#[derive(Debug, Clone)]
pub struct AuthorityTable {
    pub sid: AuthorityRef,
    pub psid: AuthorityRef,
}

#[derive(Debug, Clone)]
pub struct CPinTable {
    pub sid: CPinRef,
    pub msid: CPinRef,
}
