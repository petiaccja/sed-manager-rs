use std::sync::{Arc, Mutex};

use crate::messaging::types::{AuthorityRef, BoolOrBytes, SPRef};
use crate::messaging::uid::UID;
use crate::messaging::value::Bytes;
use crate::rpc::MethodStatus;
use crate::specification::invokers;

use super::controller::Controller;

pub struct SPSession {
    sp: SPRef,
    write: bool,
    controller: Arc<Mutex<Controller>>,
    authentications: Vec<AuthorityRef>,
}

impl SPSession {
    pub fn new(sp: SPRef, write: bool, controller: Arc<Mutex<Controller>>) -> Self {
        Self { sp, write, controller, authentications: Vec::new() }
    }

    pub fn authenticate(
        &mut self,
        invoking_id: UID,
        _authority: AuthorityRef,
        _proof: Option<Bytes>,
    ) -> Result<BoolOrBytes, MethodStatus> {
        if invoking_id != invokers::THIS_SP {
            Err(MethodStatus::InvalidParameter)
        } else {
            let controller = self.controller.lock().unwrap();
            if let Some(_sp) = controller.get_sp(self.sp) {
                Err(MethodStatus::Fail)
            } else {
                Err(MethodStatus::Fail)
            }
        }
    }
}
