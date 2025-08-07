//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::fake_device::data::byte_table::ByteTable;
use crate::fake_device::data::object_table::GenericTable;
use crate::messaging::uid::{TableUID, UID};
use crate::messaging::value::{Bytes, Named, Value};
use crate::rpc::MethodStatus;
use crate::spec::basic_types::{List, NamedValue};
use crate::spec::column_types::{
    ACERef, AuthorityRef, BoolOrBytes, BytesOrRowValues, CellBlock, CredentialRef, MethodRef,
};

pub trait SecurityProvider {
    fn get_object_table(&self, table: TableUID) -> Option<&dyn GenericTable>;
    fn get_object_table_mut(&mut self, table: TableUID) -> Option<&mut dyn GenericTable>;

    fn get_byte_table(&self, #[allow(unused)] table: TableUID) -> Option<&ByteTable> {
        None
    }

    fn get_byte_table_mut(&mut self, #[allow(unused)] table: TableUID) -> Option<&mut ByteTable> {
        None
    }

    fn authenticate(&self, authority_id: AuthorityRef, proof: Option<Bytes>) -> Result<BoolOrBytes, MethodStatus>;

    fn gen_key(
        &mut self,
        credential_id: CredentialRef,
        public_exponent: Option<u64>,
        pin_length: Option<u16>,
    ) -> Result<(), MethodStatus>;

    fn get_acl(&self, invoking_id: UID, method_id: MethodRef) -> Result<Vec<ACERef>, MethodStatus>;

    fn get(&self, invoking_id: UID, cell_block: CellBlock) -> Result<BytesOrRowValues, MethodStatus> {
        let Some(table_ref) = cell_block.target_table(invoking_id) else {
            return Err(MethodStatus::InvalidParameter);
        };

        if let Some(table) = self.get_object_table(table_ref) {
            let object_cb = cell_block.try_into_object(invoking_id).map_err(|_| MethodStatus::InvalidParameter)?;
            let object = table.get_object(object_cb.object).ok_or(MethodStatus::InvalidParameter)?;
            let first = object_cb.start_column.unwrap_or(0);
            let last = object_cb.end_column.map(|x| x + 1).unwrap_or(object.len() as u16);
            Ok(BytesOrRowValues::RowValues(
                (first..last)
                    .into_iter()
                    .map(|idx| (idx, object.get(idx as usize)))
                    .filter(|(_n, v)| !v.is_empty())
                    .map(|(n, value)| Value::from(Named { name: n.into(), value }))
                    .collect(),
            ))
        } else if let Some(table) = self.get_byte_table(table_ref) {
            let byte_cb = cell_block.try_into_byte(invoking_id).map_err(|_| MethodStatus::InvalidParameter)?;
            let first = byte_cb.start_byte.unwrap_or(0);
            let last = byte_cb.end_byte.map(|x| x + 1).unwrap_or(table.len() as u64);
            let bytes = table.read(first as usize, (last - first) as usize)?;
            Ok(BytesOrRowValues::Bytes(bytes))
        } else {
            Err(MethodStatus::InvalidParameter)
        }
    }

    fn set(
        &mut self,
        invoking_id: UID,
        where_: Option<u64>,
        values: Option<BytesOrRowValues>,
    ) -> Result<(), MethodStatus> {
        match decode_set_parameters(self, invoking_id, where_, values)? {
            DecodedSetParameters::Object { table, object, mut values } => {
                let Some(table) = self.get_object_table_mut(table) else {
                    return Err(MethodStatus::InvalidParameter);
                };
                let Some(object) = table.get_object_mut(object) else {
                    return Err(MethodStatus::InvalidParameter);
                };

                let mut rollback_idx = None;
                for (idx, (column, value)) in values.iter_mut().enumerate() {
                    let old = object.get(*column as usize);
                    if object.try_replace(*column as usize, core::mem::replace(value, old)).is_err() {
                        rollback_idx = Some(idx);
                        break;
                    }
                }

                if let Some(rollback_idx) = rollback_idx {
                    for (column, value) in values.into_iter().take(rollback_idx as usize) {
                        object.try_replace(column as usize, value).expect("error in From<Value> impl");
                    }
                    Err(MethodStatus::InvalidParameter)
                } else {
                    Ok(())
                }
            }
            DecodedSetParameters::ByteRange { table, start_byte, bytes } => {
                let Some(table) = self.get_byte_table_mut(table) else {
                    return Err(MethodStatus::InvalidParameter);
                };
                table.write(start_byte as usize, bytes.as_slice())
            }
        }
    }

    fn next(&self, table: TableUID, from: Option<UID>, count: Option<u64>) -> Result<List<UID>, MethodStatus> {
        let Some(table) = self.get_object_table(table) else {
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

enum DecodedSetParameters {
    Object { table: TableUID, object: UID, values: Vec<(u64, Value)> },
    ByteRange { table: TableUID, start_byte: u64, bytes: Vec<u8> },
}

fn decode_set_parameters<This: ?Sized>(
    this: &This,
    invoking_id: UID,
    where_: Option<u64>,
    values: Option<BytesOrRowValues>,
) -> Result<DecodedSetParameters, MethodStatus>
where
    This: SecurityProvider,
{
    if let Ok(table) = TableUID::try_from(invoking_id) {
        if let Some(_) = this.get_byte_table(table) {
            let Some(start_byte) = where_ else {
                return Err(MethodStatus::InvalidParameter);
            };
            let BytesOrRowValues::Bytes(bytes) = values.unwrap_or(BytesOrRowValues::Bytes(vec![])) else {
                return Err(MethodStatus::InvalidParameter);
            };
            Ok(DecodedSetParameters::ByteRange { table, start_byte, bytes })
        } else if let Some(_) = this.get_object_table(table) {
            let Some(object) = where_.map(|x| UID::new(x)) else {
                return Err(MethodStatus::InvalidParameter);
            };
            if object.containing_table() != Some(table.as_uid()) {
                return Err(MethodStatus::InvalidParameter);
            }
            decode_set_parameters(this, object, None, values)
        } else {
            Err(MethodStatus::InvalidParameter)
        }
    } else if let Some(table) = invoking_id.containing_table() {
        if where_.is_some() {
            return Err(MethodStatus::InvalidParameter);
        }
        let BytesOrRowValues::RowValues(row_values) = values.unwrap_or(BytesOrRowValues::RowValues(vec![])) else {
            return Err(MethodStatus::InvalidParameter);
        };
        let row_values: Result<Vec<NamedValue<u64, Value>>, _> = row_values.into_iter().map(|x| x.try_into()).collect();
        let row_values = row_values.map_err(|_| MethodStatus::InvalidParameter)?;
        let values = row_values.into_iter().map(|x| (x.name, x.value)).collect();
        Ok(DecodedSetParameters::Object { table: table.try_into().unwrap(), object: invoking_id, values })
    } else {
        Err(MethodStatus::InvalidParameter)
    }
}
