use sed_spec::objects::{AuthorityRef, CPinRef, SecurityProviderRef};

pub struct Admin {
    pub uid: SecurityProviderRef,
    pub authorities: AuthorityTable,
    pub c_pins: CPinTable,
}

pub struct AuthorityTable {
    pub sid: AuthorityRef,
    pub psid: AuthorityRef,
}

pub struct CPinTable {
    pub sid: CPinRef,
    pub msid: CPinRef,
}
