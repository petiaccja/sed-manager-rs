use as_array::AsArray;

use crate::fake_device::data::table::{AuthorityTable, CPINTable, GenericTable, SPTable, TableTable};
use crate::fake_device::{MSID_PASSWORD, PSID_PASSWORD};
use crate::messaging::uid::TableUID;
use crate::messaging::value::Bytes;
use crate::rpc::MethodStatus;
use crate::spec;
use crate::spec::column_types::{AuthMethod, AuthorityRef, BoolOrBytes, CredentialRef, LifeCycleState};
use crate::spec::objects::{Authority, CPIN, SP};
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
#[as_array_traits(GenericTable)]
pub struct SPSpecific {
    pub sp: SPTable,
}

impl AdminSP {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SecurityProvider for AdminSP {
    fn get_object_table(&self, table: TableUID) -> Option<&dyn GenericTable> {
        let basic = self.basic_sp.as_array().into_iter().find(|table_| table_.uid() == table);
        let specific = self.sp_specific.as_array().into_iter().find(|table_| table_.uid() == table);
        basic.or(specific)
    }

    fn get_object_table_mut(&mut self, table: TableUID) -> Option<&mut dyn GenericTable> {
        let basic = self.basic_sp.as_array_mut().into_iter().find(|table_| table_.uid() == table);
        let specific = self.sp_specific.as_array_mut().into_iter().find(|table_| table_.uid() == table);
        basic.or(specific)
    }

    fn authenticate(&self, authority_id: AuthorityRef, proof: Option<Bytes>) -> Result<BoolOrBytes, MethodStatus> {
        self.basic_sp.authenticate(authority_id, proof)
    }

    fn gen_key(
        &mut self,
        _credential_id: CredentialRef,
        _public_exponent: Option<u64>,
        _pin_length: Option<u16>,
    ) -> Result<(), MethodStatus> {
        Err(MethodStatus::NotAuthorized)
    }
}

impl Default for AdminSP {
    fn default() -> Self {
        let table = preconfig_table();
        let authorities = preconfig_authorities();
        let c_pin = preconfig_c_pin();
        let sp = preconfig_sp();
        Self { basic_sp: BasicSP { table, authorities, c_pin }, sp_specific: SPSpecific { sp } }
    }
}

fn preconfig_table() -> TableTable {
    TableTable::new()
}

fn preconfig_authorities() -> AuthorityTable {
    let mut authorities = AuthorityTable::new();
    let basic = [
        Authority { uid: authority::ANYBODY, name: "Anybody".into(), ..Default::default() },
        Authority { uid: authority::ADMINS, name: "Admins".into(), ..Default::default() },
        Authority { uid: authority::MAKERS, name: "Makers".into(), ..Default::default() },
        Authority {
            uid: authority::SID,
            name: "SID".into(),
            operation: AuthMethod::Password,
            credential: CredentialRef::new_other(c_pin::SID),
            ..Default::default()
        },
        Authority {
            uid: spec::psid::admin::authority::PSID,
            name: "PSID".into(),
            operation: AuthMethod::Password.into(),
            credential: CredentialRef::new_other(spec::psid::admin::c_pin::PSID),
            ..Default::default()
        },
    ];

    let admins = (1..=4).into_iter().map(|index| Authority {
        uid: authority::ADMIN.nth(index).unwrap(),
        name: format!("Admin{}", index).into(),
        enabled: false,
        operation: AuthMethod::Password,
        credential: CredentialRef::new_other(c_pin::ADMIN.nth(index).unwrap()),
        ..Default::default()
    });

    for authority in basic {
        authorities.insert(authority.uid, authority);
    }
    for authority in admins {
        authorities.insert(authority.uid, authority);
    }
    authorities
}

fn preconfig_c_pin() -> CPINTable {
    let mut c_pins = CPINTable::new();

    let basic = [
        CPIN { uid: c_pin::SID, pin: MSID_PASSWORD.into(), ..Default::default() },
        CPIN { uid: c_pin::MSID, pin: MSID_PASSWORD.into(), ..Default::default() },
        CPIN { uid: spec::psid::admin::c_pin::PSID, pin: PSID_PASSWORD.into(), ..Default::default() },
    ];
    let admins = (1..=4).into_iter().map(|index| CPIN {
        uid: c_pin::ADMIN.nth(index).unwrap(),
        pin: "8965823nz987gt346".into(),
        ..Default::default()
    });

    for pin in basic {
        c_pins.insert(pin.uid, pin);
    }
    for pin in admins {
        c_pins.insert(pin.uid, pin);
    }

    c_pins
}

fn preconfig_sp() -> SPTable {
    let mut sp = SPTable::new();
    let basic = [
        SP {
            uid: sp::ADMIN,
            name: "Admin".into(),
            life_cycle_state: LifeCycleState::Manufactured,
            ..Default::default()
        },
        SP {
            uid: sp::LOCKING,
            name: "Locking".into(),
            life_cycle_state: LifeCycleState::ManufacturedInactive,
            ..Default::default()
        },
    ];
    for spobj in basic {
        sp.insert(spobj.uid, spobj);
    }
    sp
}
