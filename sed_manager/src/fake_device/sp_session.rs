//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::collections::HashSet;

use crate::fake_device::data::object_table::AuthorityTable;
use crate::fake_device::data::security_provider::SecurityProvider;
use crate::fake_device::data::TPer;
use crate::messaging::uid::UID;
use crate::messaging::value::{Bytes, Value};
use crate::rpc::MethodStatus;
use crate::spec::basic_types::List;
use crate::spec::column_types::{
    ACERef, AuthorityRef, BoolOrBytes, BytesOrRowValues, CellBlock, CellBlockWrite, CredentialRef, MethodRef, SPRef,
};
use crate::spec::invoking_id::THIS_SP;
use crate::spec::objects::{ACEExpr as _, ACE};
use crate::spec::opal::admin::sp;
use crate::spec::{method_id, table_id};

pub struct SPSession {
    this_sp: SPRef,
    write: bool,
    authenticated: Vec<AuthorityRef>,
    reverted_sps: Vec<SPRef>, // SPs affected are added here after a call to Revert or RevertSP.
}

pub struct SPSessionExecutor<'session, 'tper> {
    session: &'session mut SPSession,
    tper: &'tper mut TPer,
}

impl SPSession {
    pub fn new(tper: &mut TPer, sp: SPRef, write: bool, host_sgn_auth: Option<AuthorityRef>) -> Self {
        use crate::spec::core::authority::ANYBODY;
        let mut new = Self { this_sp: sp, write, authenticated: vec![ANYBODY], reverted_sps: Vec::new() };
        if let Some(host_sgn_auth) = host_sgn_auth {
            new.on_tper(tper).push_authenticated(host_sgn_auth);
        };
        new
    }

    pub fn sp(&self) -> SPRef {
        self.this_sp
    }

    pub fn on_tper<'me, 'tper>(&'me mut self, tper: &'tper mut TPer) -> SPSessionExecutor<'me, 'tper> {
        SPSessionExecutor { session: self, tper }
    }

    pub fn take_reverted_sps(&mut self) -> Vec<SPRef> {
        core::mem::replace(&mut self.reverted_sps, vec![])
    }
}

impl<'session, 'tper> SPSessionExecutor<'session, 'tper> {
    pub fn authenticate(
        &mut self,
        invoking_id: UID,
        authority: AuthorityRef,
        proof: Option<Bytes>,
    ) -> Result<(BoolOrBytes,), MethodStatus> {
        if invoking_id != THIS_SP {
            return Err(MethodStatus::InvalidParameter);
        }
        let is_success = {
            let Some(security_provider) = self.tper.get_security_provider(self.session.sp()) else {
                return Err(MethodStatus::TPerMalfunction);
            };
            security_provider.authenticate(authority, proof)
        };
        if is_success == Ok(BoolOrBytes::Bool(true)) {
            self.push_authenticated(authority);
        }
        is_success.map(|out| (out,))
    }

    pub fn get(&mut self, invoking_id: UID, cell_block: CellBlock) -> Result<(BytesOrRowValues,), MethodStatus> {
        let Some(security_provider) = self.tper.get_security_provider(self.session.sp()) else {
            return Err(MethodStatus::TPerMalfunction);
        };
        if let Some((uid, columns)) = get_target_object_read(security_provider, invoking_id, &cell_block) {
            if check_authorization(security_provider, &self.session.authenticated, uid, method_id::GET, &columns) {
                security_provider.get(invoking_id, cell_block).map(|out| (out,))
            } else {
                Err(MethodStatus::NotAuthorized)
            }
        } else {
            Err(MethodStatus::InvalidParameter)
        }
    }

    pub fn set(
        &mut self,
        invoking_id: UID,
        where_: Option<u64>,
        values: Option<BytesOrRowValues>,
    ) -> Result<(), MethodStatus> {
        let Some(security_provider) = self.tper.get_security_provider_mut(self.session.sp()) else {
            return Err(MethodStatus::TPerMalfunction);
        };
        if let Some((uid, columns)) = get_target_object_write(invoking_id, where_, values.as_ref()) {
            if check_authorization(security_provider, &self.session.authenticated, uid, method_id::SET, &columns) {
                security_provider.set(invoking_id, where_, values)
            } else {
                Err(MethodStatus::NotAuthorized)
            }
        } else {
            Err(MethodStatus::InvalidParameter)
        }
    }

    pub fn next(&mut self, invoking_id: UID, from: Option<UID>, count: Option<u64>) -> Result<(List<UID>,), MethodStatus> {
        let Some(security_provider) = self.tper.get_security_provider(self.session.sp()) else {
            return Err(MethodStatus::TPerMalfunction);
        };
        let Ok(table) = invoking_id.try_into() else {
            return Err(MethodStatus::InvalidParameter);
        };
        if !check_authorization(security_provider, &self.session.authenticated, invoking_id, method_id::NEXT, &[0]) {
            return Err(MethodStatus::NotAuthorized);
        }
        security_provider.next(table, from, count).map(|out| (out,))
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
        let Some(security_provider) = self.tper.get_security_provider_mut(self.session.sp()) else {
            return Err(MethodStatus::TPerMalfunction);
        };
        security_provider.gen_key(credential_id, public_exponent, pin_length)
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
        let Some(security_provider) = self.tper.get_security_provider_mut(self.session.sp()) else {
            return Err(MethodStatus::TPerMalfunction);
        };
        security_provider.get_acl(acl_invoking_id, acl_method_id)
    }

    pub fn revert(&mut self, invoking_id: UID) -> Result<(), MethodStatus> {
        if self.session.sp() != sp::ADMIN {
            return Err(MethodStatus::NotAuthorized);
        }
        let Ok(sp) = invoking_id.try_into() else {
            return Err(MethodStatus::InvalidParameter);
        };
        self.tper.revert(sp).map(|reverted| {
            self.session.reverted_sps = reverted;
        })
    }

    pub fn revert_sp(&mut self, invoking_id: UID, keep_global_range_key: Option<bool>) -> Result<(), MethodStatus> {
        if invoking_id != THIS_SP {
            return Err(MethodStatus::InvalidParameter);
        };
        self.tper.revert_sp(self.session.sp(), keep_global_range_key).map(|reverted| {
            self.session.reverted_sps = reverted;
        })
    }

    pub fn activate(&mut self, invoking_id: UID) -> Result<(), MethodStatus> {
        if self.session.sp() != sp::ADMIN {
            return Err(MethodStatus::NotAuthorized);
        }
        let Ok(sp) = invoking_id.try_into() else {
            return Err(MethodStatus::InvalidParameter);
        };
        self.tper.activate(sp)
    }

    fn push_authenticated(&mut self, authority: AuthorityRef) {
        let class = {
            let Some(sp) = self.tper.get_security_provider(self.session.sp()) else {
                return;
            };
            let Some(auth_table) = sp.get_object_table_specific::<AuthorityTable>(table_id::AUTHORITY) else {
                return;
            };
            let Some(auth_obj) = auth_table.get(&authority) else {
                return;
            };
            auth_obj.class
        };
        if class != AuthorityRef::null() {
            self.session.authenticated.push(class);
        }
        self.session.authenticated.push(authority);
        self.session.authenticated.sort();
        self.session.authenticated.dedup();
    }
}

fn get_authorization_aces(sp: &SecurityProvider, invoking_id: UID, method_id: MethodRef) -> Vec<&ACE> {
    let Ok(ace_refs) = sp.get_acl(invoking_id, method_id) else { return vec![] }; // No ACL record -> Not authorized.
    let ace_table = sp.get_object_table(table_id::ACE).expect("invalid device configuration: ACE table does not exist");
    let mut aces = Vec::new();
    for ace_ref in ace_refs {
        let ace = ace_table
            .get_object(ace_ref.as_uid())
            .expect("invalid device configuration: ACE object does not exist");
        let ace = ace.as_any().downcast_ref::<ACE>().expect("invalid device configuration: ACE object has wrong type");
        aces.push(ace);
    }
    aces
}

fn get_target_object_read(sp: &SecurityProvider, invoking_id: UID, cell_block: &CellBlock) -> Option<(UID, Vec<u16>)> {
    let table_ref = cell_block.get_target_table(invoking_id)?;
    if let Some(table) = sp.get_object_table(table_ref) {
        let object_cb = cell_block.clone().try_into_object(invoking_id).ok()?;
        let object = table.get_object(object_cb.object)?;
        let first_column = object_cb.start_column.unwrap_or(0);
        let last_column = object_cb.end_column.map(|x| x + 1).unwrap_or(object.len() as u16);
        Some((object_cb.object, (first_column..last_column).collect()))
    } else {
        Some((table_ref.as_uid(), vec![0]))
    }
}

fn get_target_object_write(
    invoking_id: UID,
    where_: Option<u64>,
    row_values: Option<&BytesOrRowValues>,
) -> Option<(UID, Vec<u16>)> {
    match row_values {
        Some(BytesOrRowValues::RowValues(row_values)) => {
            let (_, object) = CellBlockWrite::get_target_object(invoking_id, where_.map(|value| UID::new(value)))?;
            let columns: Option<Vec<_>> = row_values
                .iter()
                .map(|value| match value {
                    Value::Named(named) => u16::try_from(&named.name).ok(),
                    _ => None,
                })
                .collect();
            Some((object, columns?))
        }
        Some(BytesOrRowValues::Bytes(_)) => Some((invoking_id.is_table().then_some(invoking_id)?, vec![0])),
        None => None,
    }
}

fn eval_authorization_aces(aces: &[&ACE], columns: &[u16], authenticated: &[AuthorityRef]) -> bool {
    let mut authorized_columns = HashSet::new();
    let mut all_columns = false;
    for ace in aces {
        let ace_expr = &ace.boolean_expr;
        let is_authorized = ace_expr.eval(authenticated).expect("invalid device configuration: ACE expression invalid");
        if is_authorized {
            if ace.columns.is_empty() {
                all_columns = true;
            }
            for column in ace.columns.iter() {
                authorized_columns.insert(column);
            }
        }
    }
    all_columns || columns.iter().all(|column| authorized_columns.contains(column))
}

fn check_authorization(
    sp: &SecurityProvider,
    authenticated: &[AuthorityRef],
    invoking_id: UID,
    method_id: MethodRef,
    columns: &[u16],
) -> bool {
    let aces = get_authorization_aces(sp, invoking_id, method_id);
    eval_authorization_aces(&aces, columns, authenticated)
}

#[cfg(test)]
mod tests {
    use crate::spec::core::authority;
    use crate::spec::objects::ace::ace_expr;

    use super::*;

    fn test_aces() -> Vec<ACE> {
        vec![
            ACE { boolean_expr: ace_expr!((authority::ANYBODY)), columns: [0, 1].into(), ..Default::default() },
            ACE { boolean_expr: ace_expr!((authority::SID)), columns: [1, 2, 3].into(), ..Default::default() },
            ACE { boolean_expr: ace_expr!((authority::RESERVE0)), columns: [].into(), ..Default::default() },
        ]
    }

    #[test]
    fn eval_authorization_aces_denied_column() {
        let aces = test_aces();
        let aces = aces.iter().collect::<Vec<_>>();
        assert_eq!(false, eval_authorization_aces(&aces, &[3, 4], &[authority::ANYBODY, authority::SID]));
    }

    #[test]
    fn eval_authorization_aces_denied_authority() {
        let aces = test_aces();
        let aces = aces.iter().collect::<Vec<_>>();
        assert_eq!(false, eval_authorization_aces(&aces, &[1], &[authority::MAKER_SYM_K]));
    }

    #[test]
    fn eval_authorization_aces_granted_all_columns() {
        let aces = test_aces();
        let aces = aces.iter().collect::<Vec<_>>();
        assert_eq!(true, eval_authorization_aces(&aces, &[1, 2], &[authority::RESERVE0]));
    }

    #[test]
    fn eval_authorization_aces_granted_single() {
        let aces = test_aces();
        let aces = aces.iter().collect::<Vec<_>>();
        assert_eq!(true, eval_authorization_aces(&aces, &[1, 2], &[authority::SID]));
    }

    #[test]
    fn eval_authorization_aces_granted_merged() {
        let aces = test_aces();
        let aces = aces.iter().collect::<Vec<_>>();
        assert_eq!(true, eval_authorization_aces(&aces, &[0, 1, 2], &[authority::ANYBODY, authority::SID]));
    }
}
