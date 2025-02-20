use crate::fake_device::data::objects::{Authority, AuthorityTable, CPin, CPinTable};
use crate::fake_device::data::table::BasicTable;
use crate::messaging::uid::TableUID;
use crate::spec::column_types::{AuthMethod, SPRef};
use crate::spec::{opal, table_id};

use super::super::SecurityProvider;

pub struct AdminSP {
    authorities: AuthorityTable,
    c_pin: CPinTable,
}

impl AdminSP {
    pub fn new() -> Self {
        Self { authorities: new_authority_table(), c_pin: new_c_pin_table() }
    }
}

impl SecurityProvider for AdminSP {
    fn uid(&self) -> SPRef {
        opal::admin::sp::ADMIN
    }

    fn get_authority_table(&self) -> Option<&AuthorityTable> {
        Some(&self.authorities)
    }

    fn get_c_pin_table(&self) -> Option<&CPinTable> {
        Some(&self.c_pin)
    }

    fn get_table(&self, uid: TableUID) -> Option<&dyn BasicTable> {
        match uid {
            table_id::AUTHORITY => Some(&self.authorities as &dyn BasicTable),
            table_id::C_PIN => Some(&self.c_pin as &dyn BasicTable),
            _ => None,
        }
    }

    fn get_table_mut(&mut self, uid: TableUID) -> Option<&mut dyn BasicTable> {
        match uid {
            table_id::AUTHORITY => Some(&mut self.authorities as &mut dyn BasicTable),
            table_id::C_PIN => Some(&mut self.c_pin as &mut dyn BasicTable),
            _ => None,
        }
    }
}

fn new_authority_table() -> AuthorityTable {
    let mut authorities = AuthorityTable::new();
    let anybody = Authority {
        name: Some("Anybody".into()),
        operation: AuthMethod::None.into(),
        credential: None,
        ..Authority::new(opal::admin::authority::ANYBODY)
    };
    let admins = Authority {
        name: Some("Admins".into()),
        enabled: true.into(),
        operation: AuthMethod::None.into(),
        credential: None,
        ..Authority::new(opal::admin::authority::ADMINS)
    };
    let makers = Authority {
        name: Some("Makers".into()),
        enabled: true.into(),
        operation: AuthMethod::None.into(),
        credential: None,
        ..Authority::new(opal::admin::authority::MAKERS)
    };
    let sid = Authority {
        name: Some("SID".into()),
        enabled: true.into(),
        operation: AuthMethod::Password.into(),
        credential: Some(opal::admin::c_pin::SID.into()),
        ..Authority::new(opal::admin::authority::SID)
    };

    authorities.0.insert(anybody.uid, anybody);
    authorities.0.insert(admins.uid, admins);
    authorities.0.insert(makers.uid, makers);
    authorities.0.insert(sid.uid, sid);

    for i in 1..=4 {
        let admin = Authority {
            name: Some(format!("Admin{}", i).into()),
            enabled: false.into(),
            operation: AuthMethod::None.into(),
            credential: Some(opal::admin::c_pin::ADMIN.nth(i).unwrap()),
            ..Authority::new(opal::admin::authority::ADMIN.nth(i).unwrap())
        };
        authorities.0.insert(admin.uid, admin);
    }

    authorities
}

fn new_c_pin_table() -> CPinTable {
    let mut c_pins = CPinTable::new();

    let sid = CPin { pin: Some("password".into()), ..CPin::new(opal::admin::c_pin::SID) };
    let msid = CPin { pin: Some("password".into()), ..CPin::new(opal::admin::c_pin::MSID) };

    c_pins.0.insert(sid.uid, sid);
    c_pins.0.insert(msid.uid, msid);

    for i in 1..=4 {
        let admin = CPin { pin: Some("password".into()), ..CPin::new(opal::admin::c_pin::ADMIN.nth(i).unwrap()) };
        c_pins.0.insert(admin.uid, admin);
    }

    c_pins
}
