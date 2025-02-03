use crate::messaging::uid::UID;
use crate::messaging::uid_range::UIDRange;

pub mod admin_authorities {
    use super::*;

    pub const ANYBODY: UID = UID::new(0x0000_0009_0000_0001);
    pub const ADMINS: UID = UID::new(0x0000_0009_0000_0002);
    pub const MAKERS: UID = UID::new(0x0000_0009_0000_0003);
    pub const SID: UID = UID::new(0x0000_0009_0000_0006);
    pub const ADMIN: UIDRange = UIDRange::new(UID::new(0x0000_0009_0000_0200), 0xFF);
}

pub mod admin_c_pins {
    use super::*;

    pub const SID: UID = UID::new(0x0000_000B_0000_0001);
    pub const MSID: UID = UID::new(0x0000_000B_0000_8402);
    pub const ADMIN: UIDRange = UIDRange::new(UID::new(0x0000_000B_0000_0200), 0xFF);
}

pub mod locking_authorities {
    use super::*;

    pub const ANYBODY: UID = UID::new(0x0000_0009_0000_0001);
    pub const ADMINS: UID = UID::new(0x0000_0009_0000_0002);
    pub const ADMIN: UIDRange = UIDRange::new(UID::new(0x0000_0009_0001_0000), 0xFFFF);
    pub const USERS: UID = UID::new(0x0000_0009_0003_0000);
    pub const USER: UIDRange = UIDRange::new(UID::new(0x0000_0009_0003_0000), 0xFFFF);
}

pub mod locking_c_pins {
    use super::*;

    pub const ADMIN: UIDRange = UIDRange::new(UID::new(0x0000_000B_0001_0000), 0xFFFF);
    pub const USER: UIDRange = UIDRange::new(UID::new(0x0000_000B_0003_0000), 0xFFFF);
}
