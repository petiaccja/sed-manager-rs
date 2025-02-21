use std::sync::{Arc, Mutex};

use crate::messaging::uid::UID;
use crate::messaging::value::Bytes;
use crate::rpc::MethodStatus;
use crate::spec::basic_types::List;
use crate::spec::column_types::{AuthorityRef, BoolOrBytes, BytesOrRowValues, CellBlock, SPRef};
use crate::spec::invoking_id::THIS_SP;
use crate::spec::opal::admin::sp;

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
        Self { this_sp: sp, write, controller, authentications: Vec::new(), reverted: Vec::new() }
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
        security_provider.next(table, from, count).map(|out| (out,))
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
