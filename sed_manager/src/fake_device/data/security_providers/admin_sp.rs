use as_array::AsArray;

use crate::fake_device::data::objects::{Authority, AuthorityTable, CPINTable, SPTable, CPIN, SP};
use crate::fake_device::data::table::BasicTable;
use crate::fake_device::MSID_PASSWORD;
use crate::messaging::uid::TableUID;
use crate::messaging::value::Bytes;
use crate::rpc::MethodStatus;
use crate::spec::column_types::{AuthMethod, AuthorityRef, BoolOrBytes, LifeCycleState};
use crate::spec::opal::admin::*;

use super::basic_sp::BasicSP;
use super::security_provider::SecurityProvider;

// Admin SP tables:
// --- Basic ---
// - Table
// - SPInfo
// - SPTemplates
// - MethodID
// - AccessControl
// - ACE
// - Authority
// - C_PIN
// --- SP-specific ---
// - TPerInfo
// - Template
// - SP
// - DataRemovalMechanism

pub struct AdminSP {
    pub basic_sp: BasicSP,
    pub sp_specific: SPSpecific,
}

#[derive(AsArray)]
#[as_array_traits(BasicTable)]
pub struct SPSpecific {
    pub sp: SPTable,
}

impl AdminSP {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SecurityProvider for AdminSP {
    fn get_table(&self, table: TableUID) -> Option<&dyn BasicTable> {
        let basic = self.basic_sp.as_array().into_iter().find(|table_| table_.uid() == table);
        let specific = self.sp_specific.as_array().into_iter().find(|table_| table_.uid() == table);
        basic.or(specific)
    }

    fn get_table_mut(&mut self, table: TableUID) -> Option<&mut dyn BasicTable> {
        let basic = self.basic_sp.as_array_mut().into_iter().find(|table_| table_.uid() == table);
        let specific = self.sp_specific.as_array_mut().into_iter().find(|table_| table_.uid() == table);
        basic.or(specific)
    }

    fn authenticate(&self, authority_id: AuthorityRef, proof: Option<Bytes>) -> Result<BoolOrBytes, MethodStatus> {
        self.basic_sp.authenticate(authority_id, proof)
    }
}

impl Default for AdminSP {
    fn default() -> Self {
        let authorities = preconfig_authorities();
        let c_pin = preconfig_c_pin();
        let sp = preconfig_sp();
        Self { basic_sp: BasicSP { authorities, c_pin }, sp_specific: SPSpecific { sp } }
    }
}

fn preconfig_authorities() -> AuthorityTable {
    let mut authorities = AuthorityTable::new();
    let anybody = Authority {
        name: Some("Anybody".into()),
        operation: AuthMethod::None.into(),
        credential: None,
        ..Authority::new(authority::ANYBODY)
    };
    let admins = Authority {
        name: Some("Admins".into()),
        enabled: true.into(),
        operation: AuthMethod::None.into(),
        credential: None,
        ..Authority::new(authority::ADMINS)
    };
    let makers = Authority {
        name: Some("Makers".into()),
        enabled: true.into(),
        operation: AuthMethod::None.into(),
        credential: None,
        ..Authority::new(authority::MAKERS)
    };
    let sid = Authority {
        name: Some("SID".into()),
        enabled: true.into(),
        operation: AuthMethod::Password.into(),
        credential: Some(c_pin::SID.into()),
        ..Authority::new(authority::SID)
    };

    authorities.insert(anybody.uid, anybody);
    authorities.insert(admins.uid, admins);
    authorities.insert(makers.uid, makers);
    authorities.insert(sid.uid, sid);

    for i in 1..=4 {
        let admin = Authority {
            name: Some(format!("Admin{}", i).into()),
            enabled: false.into(),
            operation: AuthMethod::Password.into(),
            credential: Some(c_pin::ADMIN.nth(i).unwrap()),
            ..Authority::new(authority::ADMIN.nth(i).unwrap())
        };
        authorities.insert(admin.uid, admin);
    }

    authorities
}

fn preconfig_c_pin() -> CPINTable {
    let mut c_pins = CPINTable::new();

    let sid = CPIN { pin: Some(MSID_PASSWORD.into()), ..CPIN::new(c_pin::SID) };
    let msid = CPIN { pin: Some(MSID_PASSWORD.into()), ..CPIN::new(c_pin::MSID) };

    c_pins.insert(sid.uid, sid);
    c_pins.insert(msid.uid, msid);

    for i in 1..=4 {
        let admin = CPIN { pin: Some("8965823nz987gt346".into()), ..CPIN::new(c_pin::ADMIN.nth(i).unwrap()) };
        c_pins.insert(admin.uid, admin);
    }

    c_pins
}

fn preconfig_sp() -> SPTable {
    let mut sp = SPTable::new();
    let admin = SP::new(sp::ADMIN, "Admin".into(), LifeCycleState::Manufactured);
    let locking = SP::new(sp::LOCKING, "Locking".into(), LifeCycleState::ManufacturedInactive);
    sp.insert(admin.uid, admin);
    sp.insert(locking.uid, locking);
    sp
}
