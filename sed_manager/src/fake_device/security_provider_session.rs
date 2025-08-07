//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::fake_device::data::security_providers::SecurityProvider;
use crate::messaging::uid::UID;
use crate::messaging::value::Bytes;
use crate::rpc::MethodStatus;
use crate::spec::basic_types::List;
use crate::spec::column_types::{
    ACERef, AuthorityRef, BoolOrBytes, BytesOrRowValues, CellBlock, CredentialRef, MethodRef, SPRef,
};
use crate::spec::invoking_id::THIS_SP;
use crate::spec::objects::{ACEExpr as _, ACE};
use crate::spec::opal::admin::sp;
use crate::spec::{method_id, table_id};

use super::data::OpalV2Controller;

pub struct SecurityProviderSession {
    this_sp: SPRef,
    write: bool,
    controller: Arc<Mutex<OpalV2Controller>>,
    authentications: Vec<AuthorityRef>,
    pub reverted: Vec<SPRef>, // SPs affected are added here after a call to Revert or RevertSP.
}

impl SecurityProviderSession {
    pub fn new(sp: SPRef, write: bool, controller: Arc<Mutex<OpalV2Controller>>) -> Self {
        use crate::spec::core::authority::ANYBODY;
        Self { this_sp: sp, write, controller, authentications: vec![ANYBODY], reverted: Vec::new() }
    }

    pub fn this_sp(&self) -> SPRef {
        self.this_sp
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
        let controller = self.controller.lock().unwrap();
        let Some(security_provider) = controller.get_security_provider(self.this_sp) else {
            return Err(MethodStatus::TPerMalfunction);
        };
        let result = security_provider.authenticate(authority, proof);
        if result == Ok(BoolOrBytes::Bool(true)) {
            self.authentications.push(authority);
        }
        result.map(|out| (out,))
    }

    pub fn get(&self, invoking_id: UID, cell_block: CellBlock) -> Result<(BytesOrRowValues,), MethodStatus> {
        let controller = self.controller.lock().unwrap();
        let Some(security_provider) = controller.get_security_provider(self.this_sp) else {
            return Err(MethodStatus::TPerMalfunction);
        };
        let (uid, columns) = get_target_object(security_provider, invoking_id, &cell_block);
        if !check_authorization(security_provider, &self.authentications, uid, method_id::GET, &columns) {
            return Err(MethodStatus::NotAuthorized);
        }
        security_provider.get(invoking_id, cell_block).map(|out| (out,))
    }

    pub fn set(
        &mut self,
        invoking_id: UID,
        where_: Option<u64>,
        values: Option<BytesOrRowValues>,
    ) -> Result<(), MethodStatus> {
        let mut controller = self.controller.lock().unwrap();
        let Some(security_provider) = controller.get_security_provider_mut(self.this_sp) else {
            return Err(MethodStatus::TPerMalfunction);
        };
        security_provider.set(invoking_id, where_, values)
    }

    pub fn next(&self, invoking_id: UID, from: Option<UID>, count: Option<u64>) -> Result<(List<UID>,), MethodStatus> {
        let controller = self.controller.lock().unwrap();
        let Some(security_provider) = controller.get_security_provider(self.this_sp) else {
            return Err(MethodStatus::TPerMalfunction);
        };
        let Ok(table) = invoking_id.try_into() else {
            return Err(MethodStatus::InvalidParameter);
        };
        if !check_authorization(security_provider, &self.authentications, invoking_id, method_id::NEXT, &[0]) {
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
        let mut controller = self.controller.lock().unwrap();
        let Some(security_provider) = controller.get_security_provider_mut(self.this_sp) else {
            return Err(MethodStatus::TPerMalfunction);
        };
        security_provider.gen_key(credential_id, public_exponent, pin_length)
    }

    pub fn get_acl(
        &self,
        invoking_id: UID,
        acl_invoking_id: UID,
        acl_method_id: MethodRef,
    ) -> Result<Vec<ACERef>, MethodStatus> {
        if invoking_id != table_id::ACCESS_CONTROL.as_uid() {
            return Err(MethodStatus::InvalidParameter);
        }
        let mut controller = self.controller.lock().unwrap();
        let Some(security_provider) = controller.get_security_provider_mut(self.this_sp) else {
            return Err(MethodStatus::TPerMalfunction);
        };
        security_provider.get_acl(acl_invoking_id, acl_method_id)
    }

    pub fn revert(&mut self, invoking_id: UID) -> Result<(), MethodStatus> {
        let mut controller = self.controller.lock().unwrap();
        if self.this_sp != sp::ADMIN {
            return Err(MethodStatus::NotAuthorized);
        }
        let Ok(sp) = invoking_id.try_into() else {
            return Err(MethodStatus::InvalidParameter);
        };
        controller.revert(sp).map(|reverted| {
            self.reverted = reverted;
            ()
        })
    }

    pub fn revert_sp(&mut self, invoking_id: UID, keep_global_range_key: Option<bool>) -> Result<(), MethodStatus> {
        let mut controller = self.controller.lock().unwrap();
        if invoking_id != THIS_SP {
            return Err(MethodStatus::InvalidParameter);
        };
        controller.revert_sp(self.this_sp, keep_global_range_key).map(|reverted| {
            self.reverted = reverted;
            ()
        })
    }

    pub fn activate(&self, invoking_id: UID) -> Result<(), MethodStatus> {
        let mut controller = self.controller.lock().unwrap();
        if self.this_sp != sp::ADMIN {
            return Err(MethodStatus::NotAuthorized);
        }
        let Ok(sp) = invoking_id.try_into() else {
            return Err(MethodStatus::InvalidParameter);
        };
        controller.activate(sp)
    }
}

fn get_authorization_aces(sp: &dyn SecurityProvider, invoking_id: UID, method_id: MethodRef) -> Vec<&ACE> {
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

fn get_target_object(sp: &dyn SecurityProvider, invoking_id: UID, cell_block: &CellBlock) -> (UID, Vec<u16>) {
    let Some(table_ref) = cell_block.target_table(invoking_id) else {
        return (UID::null(), vec![]);
    };
    if let Some(table) = sp.get_object_table(table_ref) {
        let Ok(object_cb) = cell_block.clone().try_into_object(invoking_id) else {
            return (UID::null(), vec![]);
        };
        let Some(object) = table.get_object(object_cb.object) else {
            return (UID::null(), vec![]);
        };
        let first_column = object_cb.start_column.unwrap_or(0);
        let last_column = object_cb.end_column.map(|x| x + 1).unwrap_or(object.len() as u16);
        (object_cb.object, (first_column..last_column).collect())
    } else {
        (table_ref.as_uid(), vec![0])
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
    sp: &dyn SecurityProvider,
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
