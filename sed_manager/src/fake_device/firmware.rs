//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::fake_device::data::object_table::AuthorityTable;
use crate::fake_device::data::security_provider::SecurityProvider;
use crate::fake_device::data::{object_table::CPINTable, TPer};
use crate::fake_device::transient::Transient;
use crate::messaging::uid::UID;
use crate::messaging::value::{Bytes, Value};
use crate::rpc::{MethodStatus, Properties, SessionIdentifier};
use crate::spec::basic_types::{List, NamedValue};
use crate::spec::column_types::{
    ACERef, AuthorityRef, BoolOrBytes, BytesOrRowValues, CellBlock, CellBlockWrite, CredentialRef, LifeCycleState,
    MaxBytes32, MethodRef, SPRef,
};
use crate::spec::invoking_id::THIS_SP;
use crate::spec::table_id;
use crate::spec::{self, method_id};

pub struct Firmware {
    pub tper: TPer,
    pub transient: Transient,
    pruned_session_ids: Vec<SessionIdentifier>,
}

pub struct SPSession<'fw> {
    session_id: SessionIdentifier,
    firmware: &'fw mut Firmware,
}

impl Firmware {
    pub fn new(tper: TPer, capabilities: Properties) -> Self {
        Self { tper, transient: Transient::new(capabilities), pruned_session_ids: Vec::new() }
    }

    pub fn sp_session<'me>(&'me mut self, session_id: SessionIdentifier) -> Option<SPSession<'me>> {
        self.transient.get_session(session_id)?;
        Some(SPSession { session_id, firmware: self })
    }

    pub fn take_pruned_session_ids(&mut self) -> Vec<SessionIdentifier> {
        core::mem::replace(&mut self.pruned_session_ids, vec![])
    }

    pub fn properties(
        &mut self,
        host_properties: Option<List<NamedValue<MaxBytes32, u32>>>,
    ) -> Result<(List<NamedValue<MaxBytes32, u32>>, Option<List<NamedValue<MaxBytes32, u32>>>), MethodStatus> {
        let host_properties = host_properties.unwrap_or(List::new());
        let host_properties = Properties::from_list(host_properties.as_slice());
        let common_properties = Properties::common(&self.transient.capabilities, &host_properties);
        self.transient.properties = common_properties.clone();
        Ok((self.transient.capabilities.to_list(), Some(common_properties.to_list())))
    }

    pub fn start_session(
        &mut self,
        hsn: u32,
        sp_uid: SPRef,
        _write: bool,
        host_challenge: Option<Bytes>,
        _host_exch_auth: Option<AuthorityRef>,
        _host_exch_cert: Option<Bytes>,
        host_sgn_auth: Option<AuthorityRef>,
        _host_sgn_cert: Option<Bytes>,
        _session_timeout: Option<u32>,
        _trans_timeout: Option<u32>,
        _initial_credit: Option<u32>,
        _signed_hash: Option<Bytes>,
    ) -> Result<
        (u32, u32, Option<Bytes>, Option<Bytes>, Option<Bytes>, Option<u32>, Option<u32>, Option<Bytes>),
        MethodStatus,
    > {
        let session_id = self.transient.add_session(sp_uid, hsn);
        if let Some(authority) = host_sgn_auth {
            if let Err(err) = self.sp_session(session_id).unwrap().authenticate(THIS_SP, authority, host_challenge) {
                self.transient.remove_session(session_id);
                return Err(err);
            }
        }
        Ok((session_id.hsn, session_id.tsn, None, None, None, None, None, None))
    }
}

impl<'fw> SPSession<'fw> {
    pub fn activate(&mut self, _invoking_id: UID, sp_ref: SPRef) -> Result<(), MethodStatus> {
        if self.firmware.tper.get_life_cycle_state(sp_ref) != Ok(LifeCycleState::ManufacturedInactive) {
            return Err(MethodStatus::InvalidParameter);
        }
        self.firmware.tper.set_life_cycle_state(sp_ref, LifeCycleState::Manufactured)?;

        // Copy PINs from Admin SP.
        let sid_c_pin_value = {
            let Some(admin_sp) = self.firmware.tper.get_admin_sp() else {
                return Ok(()); // No Admin SP, nothing to copy.
            };
            let Some(admin_c_pins) = admin_sp.get_object_table_specific::<CPINTable>(table_id::C_PIN) else {
                return Ok(());
            };
            let Some(sid_c_pin) = admin_c_pins.get(&spec::opal::admin::c_pin::SID) else {
                return Ok(());
            };
            sid_c_pin.pin.clone()
        };

        let activated_sp = self.firmware.tper.get_sp_mut(sp_ref).ok_or(MethodStatus::InvalidParameter)?;
        let Some(activated_c_pins) = activated_sp.get_object_table_specific_mut::<CPINTable>(table_id::C_PIN) else {
            return Ok(());
        };
        for c_pin in activated_c_pins.values_mut() {
            c_pin.pin = sid_c_pin_value.clone();
        }
        Ok(())
    }

    pub fn revert(&mut self, invoking_id: UID, sp: SPRef) -> Result<(), MethodStatus> {
        self.revert_sp(invoking_id, sp, None)
    }

    pub fn revert_sp(
        &mut self,
        _invoking_id: UID,
        sp_ref: SPRef,
        _keep_global_range_key: Option<bool>,
    ) -> Result<(), MethodStatus> {
        let admin_sp_ref = self.firmware.tper.get_admin_sp_uid().ok_or(MethodStatus::TPerMalfunction)?;
        if sp_ref == admin_sp_ref {
            // Revert all security providers.
            for (sp_ref, sp) in &mut self.firmware.tper.security_providers {
                *sp = (self.firmware.tper.sp_factory)(*sp_ref);
                self.firmware.pruned_session_ids.append(&mut self.firmware.transient.prune_sessions(*sp_ref));
            }
            Ok(())
        } else {
            // Revert only the specified security provider.
            self.firmware.tper.set_life_cycle_state(sp_ref, LifeCycleState::Manufactured)?;
            let reset_sp = (self.firmware.tper.sp_factory)(sp_ref);
            *self.firmware.tper.get_sp_mut(sp_ref).ok_or(MethodStatus::InvalidParameter)? = reset_sp;
            self.firmware.pruned_session_ids.append(&mut self.firmware.transient.prune_sessions(sp_ref));
            Ok(())
        }
    }

    pub fn authenticate(
        &mut self,
        invoking_id: UID,
        authority: AuthorityRef,
        proof: Option<Bytes>,
    ) -> Result<(BoolOrBytes,), MethodStatus> {
        if invoking_id != THIS_SP {
            return Err(MethodStatus::InvalidParameter);
        }
        let is_success = self.this_sp()?.authenticate(authority, proof);
        if is_success == Ok(BoolOrBytes::Bool(true)) {
            self.commit_authentication(authority)?;
        }
        is_success.map(|out| (out,))
    }

    pub fn get(&mut self, invoking_id: UID, cell_block: CellBlock) -> Result<(BytesOrRowValues,), MethodStatus> {
        let (target_uid, target_columns) = self.get_cell_block_target(invoking_id, &cell_block)?;
        if self.is_authorized(target_uid, method_id::GET, &target_columns) {
            let security_provider = self.this_sp()?;
            security_provider.get(invoking_id, cell_block).map(|out| (out,))
        } else {
            Err(MethodStatus::NotAuthorized)
        }
    }

    pub fn set(
        &mut self,
        invoking_id: UID,
        where_: Option<u64>,
        values: Option<BytesOrRowValues>,
    ) -> Result<(), MethodStatus> {
        let (target_uid, target_columns) = Self::get_set_target(invoking_id, where_, values.as_ref())?;
        if self.is_authorized(target_uid, method_id::SET, &target_columns) {
            let security_provider = self.this_sp_mut()?;
            security_provider.set(invoking_id, where_, values)
        } else {
            Err(MethodStatus::NotAuthorized)
        }
    }

    pub fn next(
        &mut self,
        invoking_id: UID,
        from: Option<UID>,
        count: Option<u64>,
    ) -> Result<(List<UID>,), MethodStatus> {
        let Ok(table) = invoking_id.try_into() else {
            return Err(MethodStatus::InvalidParameter);
        };
        if self.is_authorized(invoking_id, method_id::NEXT, &[0]) {
            let sp = self.this_sp()?;
            sp.next(table, from, count).map(|out| (out,))
        } else {
            return Err(MethodStatus::NotAuthorized);
        }
    }

    pub fn gen_key(
        &mut self,
        invoking_id: UID,
        public_exponent: Option<u64>,
        pin_length: Option<u16>,
    ) -> Result<(), MethodStatus> {
        let Ok(credential_id) = CredentialRef::try_from(invoking_id) else {
            return Err(MethodStatus::InvalidParameter);
        };
        let sp = self.this_sp_mut()?;
        sp.gen_key(credential_id, public_exponent, pin_length)
    }

    pub fn get_acl(
        &mut self,
        invoking_id: UID,
        acl_invoking_id: UID,
        acl_method_id: MethodRef,
    ) -> Result<Vec<ACERef>, MethodStatus> {
        if invoking_id != table_id::ACCESS_CONTROL.as_uid() {
            return Err(MethodStatus::InvalidParameter);
        }
        let sp = self.this_sp_mut()?;
        sp.get_acl(acl_invoking_id, acl_method_id)
    }

    fn this_sp(&self) -> Result<&SecurityProvider, MethodStatus> {
        let state = self.firmware.transient.get_session(self.session_id).ok_or(MethodStatus::Fail)?;
        self.firmware.tper.get_sp(state.sp).ok_or(MethodStatus::TPerMalfunction)
    }

    fn this_sp_mut(&mut self) -> Result<&mut SecurityProvider, MethodStatus> {
        let state = self.firmware.transient.get_session(self.session_id).ok_or(MethodStatus::Fail)?;
        self.firmware.tper.get_sp_mut(state.sp).ok_or(MethodStatus::TPerMalfunction)
    }

    fn is_authorized(&self, invoking_id: UID, method_id: MethodRef, columns: &[u16]) -> bool {
        let Some(state) = self.firmware.transient.get_session(self.session_id) else {
            return false;
        };
        let Some(sp) = self.firmware.tper.get_sp(state.sp) else {
            return false;
        };
        sp.is_authorized(&state.authenticated, invoking_id, method_id, columns)
    }

    fn get_cell_block_target(&self, invoking_id: UID, cell_block: &CellBlock) -> Result<(UID, Vec<u16>), MethodStatus> {
        let table_ref = cell_block.get_target_table(invoking_id).ok_or(MethodStatus::InvalidParameter)?;
        let sp = self.this_sp()?;
        if let Some(table) = sp.get_object_table(table_ref) {
            let object_cb =
                cell_block.clone().try_into_object(invoking_id).map_err(|_| MethodStatus::InvalidParameter)?;
            let object = table.get_object(object_cb.object).ok_or(MethodStatus::InvalidParameter)?;
            let first_column = object_cb.start_column.unwrap_or(0);
            let last_column = object_cb.end_column.map(|x| x + 1).unwrap_or(object.len() as u16);
            Ok((object_cb.object, (first_column..last_column).collect()))
        } else {
            Ok((table_ref.as_uid(), vec![0]))
        }
    }

    fn get_set_target(
        invoking_id: UID,
        where_: Option<u64>,
        row_values: Option<&BytesOrRowValues>,
    ) -> Result<(UID, Vec<u16>), MethodStatus> {
        match row_values {
            Some(BytesOrRowValues::RowValues(row_values)) => {
                let (_, object) = CellBlockWrite::get_target_object(invoking_id, where_.map(|value| UID::new(value)))
                    .ok_or(MethodStatus::InvalidParameter)?;
                let columns: Option<Vec<_>> = row_values
                    .iter()
                    .map(|value| match value {
                        Value::Named(named) => u16::try_from(&named.name).ok(),
                        _ => None,
                    })
                    .collect();
                Ok((object, columns.ok_or(MethodStatus::InvalidParameter)?))
            }
            Some(BytesOrRowValues::Bytes(_)) => {
                Ok((invoking_id.is_table().then_some(invoking_id).ok_or(MethodStatus::InvalidParameter)?, vec![0]))
            }
            None => Err(MethodStatus::InvalidParameter),
        }
    }

    fn commit_authentication(&mut self, authority: AuthorityRef) -> Result<(), MethodStatus> {
        let sp = self.this_sp()?;
        let auth_table = sp
            .get_object_table_specific::<AuthorityTable>(table_id::AUTHORITY)
            .ok_or(MethodStatus::TPerMalfunction)?;
        let auth_obj = auth_table.get(&authority).ok_or(MethodStatus::InvalidParameter)?;
        let class = auth_obj.class;
        let session = self.firmware.transient.get_session_mut(self.session_id).ok_or(MethodStatus::Fail)?;
        if class != AuthorityRef::null() {
            session.authenticated.push(class);
        }
        session.authenticated.push(authority);
        session.authenticated.sort();
        session.authenticated.dedup();
        Ok(())
    }
}
