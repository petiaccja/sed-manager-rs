use as_array::AsArray;

use crate::fake_device::data::objects::{Authority, AuthorityTable, CPINTable, CPIN};
use crate::fake_device::data::table::BasicTable;
use crate::messaging::uid::TableUID;
use crate::messaging::value::Bytes;
use crate::rpc::MethodStatus;
use crate::spec::column_types::{AuthMethod, AuthorityRef, BoolOrBytes};
use crate::spec::opal;

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
    sp_specific: SPSpecific,
}

#[derive(AsArray)]
#[as_array_traits(BasicTable)]
struct SPSpecific {}

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
        Self { basic_sp: BasicSP { authorities, c_pin }, sp_specific: SPSpecific {} }
    }
}

fn preconfig_authorities() -> AuthorityTable {
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

    authorities.insert(anybody.uid, anybody);
    authorities.insert(admins.uid, admins);
    authorities.insert(makers.uid, makers);
    authorities.insert(sid.uid, sid);

    for i in 1..=4 {
        let admin = Authority {
            name: Some(format!("Admin{}", i).into()),
            enabled: false.into(),
            operation: AuthMethod::None.into(),
            credential: Some(opal::admin::c_pin::ADMIN.nth(i).unwrap()),
            ..Authority::new(opal::admin::authority::ADMIN.nth(i).unwrap())
        };
        authorities.insert(admin.uid, admin);
    }

    authorities
}

fn preconfig_c_pin() -> CPINTable {
    let mut c_pins = CPINTable::new();

    let sid = CPIN { pin: Some("password".into()), ..CPIN::new(opal::admin::c_pin::SID) };
    let msid = CPIN { pin: Some("password".into()), ..CPIN::new(opal::admin::c_pin::MSID) };

    c_pins.insert(sid.uid, sid);
    c_pins.insert(msid.uid, msid);

    for i in 1..=4 {
        let admin = CPIN { pin: Some("password".into()), ..CPIN::new(opal::admin::c_pin::ADMIN.nth(i).unwrap()) };
        c_pins.insert(admin.uid, admin);
    }

    c_pins
}
