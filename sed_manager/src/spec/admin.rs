use sed_spec::objects::{AuthorityRef, CPinRef, SecurityProviderRef};

#[derive(Debug)]
pub struct Admin {
    pub uid: SecurityProviderRef,
    pub authorities: AuthorityTable,
    pub c_pins: CPinTable,
}

#[derive(Debug)]
pub struct AuthorityTable {
    pub sid: AuthorityRef,
    pub psid: AuthorityRef,
}

#[derive(Debug)]
pub struct CPinTable {
    pub sid: CPinRef,
    pub msid: CPinRef,
}
