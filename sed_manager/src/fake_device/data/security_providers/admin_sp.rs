use crate::fake_device::data::objects::{Authority, AuthorityTable, CPin, CPinTable};
use crate::fake_device::data::table::BasicTable;
use crate::fake_device::data::Object;
use crate::messaging::uid::UID;
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
        opal::admin::sp::ADMIN.into()
    }

    fn get_authority_table(&self) -> Option<&AuthorityTable> {
        Some(&self.authorities)
    }

    fn get_c_pin_table(&self) -> Option<&CPinTable> {
        Some(&self.c_pin)
    }

    fn get_table(&self, uid: UID) -> Option<&dyn BasicTable> {
        match uid {
            table_id::AUTHORITY => Some(&self.authorities as &dyn BasicTable),
            table_id::C_PIN => Some(&self.c_pin as &dyn BasicTable),
            _ => None,
        }
    }

    fn get_table_mut(&mut self, uid: UID) -> Option<&mut dyn BasicTable> {
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
        uid: opal::admin::authority::ANYBODY.into(),
        name: Some("Anybody".into()),
        operation: AuthMethod::None.into(),
        credential: None,
        ..Default::default()
    };
    let admins = Authority {
        uid: opal::admin::authority::ADMINS.into(),
        name: Some("Admins".into()),
        enabled: true.into(),
        operation: AuthMethod::None.into(),
        credential: None,
        ..Default::default()
    };
    let makers = Authority {
        uid: opal::admin::authority::MAKERS.into(),
        name: Some("Makers".into()),
        enabled: true.into(),
        operation: AuthMethod::None.into(),
        credential: None,
        ..Default::default()
    };
    let sid = Authority {
        uid: opal::admin::authority::SID.into(),
        name: Some("SID".into()),
        enabled: true.into(),
        operation: AuthMethod::Password.into(),
        credential: Some(opal::admin::c_pin::SID.into()),
        ..Default::default()
    };

    authorities.0.insert(anybody.uid().into(), anybody);
    authorities.0.insert(admins.uid().into(), admins);
    authorities.0.insert(makers.uid().into(), makers);
    authorities.0.insert(sid.uid().into(), sid);

    for i in 1..=4 {
        let admin = Authority {
            uid: opal::admin::authority::ADMIN.nth(i).unwrap().into(),
            name: Some(format!("Admin{}", i).into()),
            enabled: false.into(),
            operation: AuthMethod::None.into(),
            credential: Some(opal::admin::c_pin::ADMIN.nth(i).unwrap().into()),
            ..Default::default()
        };
        authorities.0.insert(admin.uid().into(), admin);
    }

    authorities
}

fn new_c_pin_table() -> CPinTable {
    let mut c_pins = CPinTable::new();

    let sid = CPin { uid: opal::admin::c_pin::SID.into(), pin: Some("password".into()), ..Default::default() };
    let msid = CPin { uid: opal::admin::c_pin::MSID.into(), pin: Some("password".into()), ..Default::default() };

    c_pins.0.insert(sid.uid().into(), sid);
    c_pins.0.insert(msid.uid().into(), msid);

    for i in 1..=4 {
        let admin = CPin {
            uid: opal::admin::c_pin::ADMIN.nth(i).unwrap().into(),
            pin: Some("password".into()),
            ..Default::default()
        };
        c_pins.0.insert(admin.uid().into(), admin);
    }

    c_pins
}
