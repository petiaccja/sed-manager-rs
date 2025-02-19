use std::sync::{Arc, Mutex};

use crate::messaging::uid::UID;
use crate::messaging::value::{Bytes, Named, Value};
use crate::rpc::MethodStatus;
use crate::spec::basic_types::{List, NamedValue, ObjectReference};
use crate::spec::column_types::{AuthorityRef, BoolOrBytes, BytesOrRowValues, CellBlock, Password, SPRef};
use crate::spec::{invoking_id, table_id};

use super::data::SSC;

pub struct SPSession {
    sp: SPRef,
    write: bool,
    ssc: Arc<Mutex<SSC>>,
    authentications: Vec<AuthorityRef>,
}

impl SPSession {
    pub fn new(sp: SPRef, write: bool, controller: Arc<Mutex<SSC>>) -> Self {
        Self { sp, write, ssc: controller, authentications: Vec::new() }
    }

    pub fn sp(&self) -> SPRef {
        self.sp
    }

    pub fn authenticate(
        &self,
        invoking_id: UID,
        authority: AuthorityRef,
        proof: Option<Bytes>,
    ) -> Result<BoolOrBytes, MethodStatus> {
        if invoking_id != invoking_id::THIS_SP {
            return Err(MethodStatus::InvalidParameter);
        };
        let ssc = self.ssc.lock().unwrap();
        let Some(sp) = ssc.get_sp(self.sp) else {
            return Err(MethodStatus::TPerMalfunction);
        };
        let Some(authority_table) = sp.get_authority_table() else {
            return Err(MethodStatus::InvalidParameter);
        };
        let Some(authority_obj) = authority_table.0.get(&authority) else {
            return Err(MethodStatus::InvalidParameter);
        };
        let Some(credential_ref) = authority_obj.credential else {
            return Ok(BoolOrBytes::Bool(true));
        };
        if credential_ref.containing_table().unwrap() == table_id::C_PIN {
            let Some(c_pin_table) = sp.get_c_pin_table() else {
                return Err(MethodStatus::TPerMalfunction);
            };
            if let Some(credential_obj) = c_pin_table.0.get(&credential_ref) {
                let pw_correct =
                    credential_obj.pin.as_ref().unwrap_or(&Password::default()).0 == proof.unwrap_or(vec![]);
                Ok(BoolOrBytes::Bool(pw_correct))
            } else {
                Err(MethodStatus::TPerMalfunction)
            }
        } else {
            Err(MethodStatus::TPerMalfunction)
        }
    }

    pub fn get(&self, invoking_id: UID, cell_block: CellBlock) -> Result<BytesOrRowValues, MethodStatus> {
        if invoking_id.is_table() {
            // FakeDevice only supports calling `Get` on a byte table.
            Err(MethodStatus::InvalidParameter)
        } else {
            let table = invoking_id.containing_table().expect("must be dealing with an object in the else branch");
            let ssc = self.ssc.lock().unwrap();
            let Some(sp) = ssc.get_sp(self.sp) else {
                return Err(MethodStatus::TPerMalfunction);
            };
            let Some(table) = sp.get_table(table) else {
                return Err(MethodStatus::InvalidParameter);
            };
            let Some(object) = table.get_object(invoking_id.into()) else {
                return Err(MethodStatus::InvalidParameter);
            };
            let first = cell_block.start_column.unwrap_or(0);
            let last = cell_block.end_column.map(|x| x + 1).unwrap_or(object.len() as u16);
            Ok(BytesOrRowValues::RowValues(
                (first..last)
                    .into_iter()
                    .map(|idx| (idx, object.get_column(idx as usize)))
                    .filter(|(_n, v)| !v.is_empty())
                    .map(|(n, value)| Value::from(Named { name: n.into(), value }))
                    .collect(),
            ))
        }
    }

    pub fn set(
        &mut self,
        invoking_id: UID,
        where_: Option<u64>,
        values: Option<BytesOrRowValues>,
    ) -> Result<(), MethodStatus> {
        if invoking_id.is_table() {
            // FakeDevice only supports calling `Set` on a byte table.
            let _ = where_;
            Err(MethodStatus::InvalidParameter)
        } else {
            let table = invoking_id.containing_table().expect("must be dealing with an object in the else branch");
            let mut ssc = self.ssc.lock().unwrap();
            let Some(sp) = ssc.get_sp_mut(self.sp) else {
                return Err(MethodStatus::TPerMalfunction);
            };
            let Some(table) = sp.get_table_mut(table) else {
                return Err(MethodStatus::InvalidParameter);
            };
            let Some(object) = table.get_object_mut(invoking_id.into()) else {
                return Err(MethodStatus::InvalidParameter);
            };
            let BytesOrRowValues::RowValues(row_values) = values.unwrap_or(BytesOrRowValues::RowValues(vec![])) else {
                return Err(MethodStatus::InvalidParameter);
            };
            let Ok(mut nvps) =
                row_values.into_iter().map(|x| NamedValue::<u64, Value>::try_from(x)).collect::<Result<Vec<_>, _>>()
            else {
                return Err(MethodStatus::InvalidParameter);
            };

            let mut rollback_idx = None;
            for (idx, NamedValue { name, value }) in nvps.iter_mut().enumerate() {
                let old = object.get_column(*name as usize);
                if object.try_set_column(*name as usize, std::mem::replace(value, old)).is_err() {
                    rollback_idx = Some(idx);
                    break;
                }
            }

            if let Some(rollback_idx) = rollback_idx {
                for NamedValue { name, value } in nvps.into_iter().take(rollback_idx as usize) {
                    object.try_set_column(name as usize, value).expect("error in From<Value> impl");
                }
                return Err(MethodStatus::InvalidParameter);
            } else {
                Ok(())
            }
        }
    }

    pub fn next(
        &self,
        invoking_id: UID,
        from: Option<ObjectReference>,
        count: Option<u64>,
    ) -> Result<List<ObjectReference>, MethodStatus> {
        let mut ssc = self.ssc.lock().unwrap();
        let Some(sp) = ssc.get_sp_mut(self.sp) else {
            return Err(MethodStatus::TPerMalfunction);
        };
        let Some(table) = sp.get_table_mut(invoking_id) else {
            return Err(MethodStatus::InvalidParameter);
        };

        let next = (0..=count.unwrap_or(1))
            .into_iter()
            .scan(from, |from, _| std::mem::replace(from, table.next_from(*from)))
            .skip(1);
        Ok(List(next.collect()))
    }
}
