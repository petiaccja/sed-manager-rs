use crate::fake_device::data::table::GenericTable;
use crate::messaging::uid::{TableUID, UID};
use crate::messaging::value::{Bytes, Named, Value};
use crate::rpc::MethodStatus;
use crate::spec;
use crate::spec::basic_types::{List, NamedValue};
use crate::spec::column_types::{
    ACERef, AuthorityRef, BoolOrBytes, BytesOrRowValues, CellBlock, CredentialRef, MethodRef,
};

pub trait SecurityProvider {
    fn get_table(&self, table: TableUID) -> Option<&dyn GenericTable>;
    fn get_table_mut(&mut self, table: TableUID) -> Option<&mut dyn GenericTable>;
    fn authenticate(&self, authority_id: AuthorityRef, proof: Option<Bytes>) -> Result<BoolOrBytes, MethodStatus>;
    fn gen_key(
        &mut self,
        credential_id: CredentialRef,
        public_exponent: Option<u64>,
        pin_length: Option<u16>,
    ) -> Result<(), MethodStatus>;

    fn get_acl(&self, _invoking_id: UID, _method_id: MethodRef) -> Result<List<ACERef>, MethodStatus> {
        // TODO: implement this later if ever needed for testing.
        Ok(vec![].into())
    }

    fn get(&self, target: UID, cell_block: CellBlock) -> Result<BytesOrRowValues, MethodStatus> {
        if target.is_table() {
            Err(MethodStatus::Fail) // TODO: Implement for byte tables.
        } else if let Some(table_id) = target.containing_table() {
            let Some(table) = self.get_table(table_id.try_into().unwrap()) else {
                return Err(MethodStatus::InvalidParameter);
            };
            let Some(object) = table.get_object(target) else {
                return Err(MethodStatus::InvalidParameter);
            };
            let first = cell_block.start_column.unwrap_or(0);
            let last = cell_block.end_column.map(|x| x + 1).unwrap_or(object.len() as u16);
            Ok(BytesOrRowValues::RowValues(
                (first..last)
                    .into_iter()
                    .map(|idx| (idx, object.get(idx as usize)))
                    .filter(|(_n, v)| !v.is_empty())
                    .map(|(n, value)| Value::from(Named { name: n.into(), value }))
                    .collect(),
            ))
        } else {
            Err(MethodStatus::InvalidParameter)
        }
    }

    fn set(&mut self, target: UID, where_: Option<u64>, values: Option<BytesOrRowValues>) -> Result<(), MethodStatus> {
        if target == spec::opal::locking::locking::RANGE.nth(8).unwrap().as_uid() {
            return Err(MethodStatus::NotAuthorized); // For testing purposes.
        }
        if target.is_table() {
            let _ = where_;
            Err(MethodStatus::InvalidParameter) // TODO: Implement for byte tables.
        } else if let Some(table_id) = target.containing_table() {
            let Some(table) = self.get_table_mut(table_id.try_into().unwrap()) else {
                return Err(MethodStatus::InvalidParameter);
            };
            let Some(object) = table.get_object_mut(target) else {
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
                let old = object.get(*name as usize);
                if object.try_replace(*name as usize, core::mem::replace(value, old)).is_err() {
                    rollback_idx = Some(idx);
                    break;
                }
            }

            if let Some(rollback_idx) = rollback_idx {
                for NamedValue { name, value } in nvps.into_iter().take(rollback_idx as usize) {
                    object.try_replace(name as usize, value).expect("error in From<Value> impl");
                }
                return Err(MethodStatus::InvalidParameter);
            } else {
                Ok(())
            }
        } else {
            Err(MethodStatus::InvalidParameter)
        }
    }

    fn next(&self, table: TableUID, from: Option<UID>, count: Option<u64>) -> Result<List<UID>, MethodStatus> {
        let Some(table) = self.get_table(table) else {
            return Err(MethodStatus::InvalidParameter);
        };
        let mut uids = Vec::new();
        let mut last = from;
        while let Some(uid) = table.next_from(last) {
            if uids.len() as u64 >= count.unwrap_or(u64::MAX) {
                break;
            }
            last = Some(uid);
            uids.push(uid);
        }
        Ok(List(uids))
    }
}
