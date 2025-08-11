//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::collections::{HashMap, HashSet};

use crate::fake_device::data::access_control_table::AccessControlTable;
use crate::fake_device::data::byte_table::ByteTable;
use crate::fake_device::data::object_table::{ACETable, AuthorityTable, CPINTable, GenericTable, KAES256Table};
use crate::messaging::uid::{TableUID, UID};
use crate::messaging::value::{Bytes, Named, Value};
use crate::rpc::MethodStatus;
use crate::spec::basic_types::List;
use crate::spec::column_types::{
    ACERef, AuthorityRef, BoolOrBytes, BytesOrRowValues, CPINRef, CellBlock, CellBlockWrite, CredentialRef, KAES256Ref,
    Key256, MethodRef,
};
use crate::spec::objects::{ACEExpr as _, ACE};
use crate::spec::table_id;

pub struct SecurityProvider {
    pub access_control: AccessControlTable,
    pub object_tables: HashMap<TableUID, Box<dyn GenericTable>>,
    pub byte_tables: HashMap<TableUID, ByteTable>,
}

impl SecurityProvider {
    pub fn get_object_table(&self, table: TableUID) -> Option<&dyn GenericTable> {
        self.object_tables.get(&table).map(|x| x.as_ref())
    }

    pub fn get_object_table_mut(&mut self, table: TableUID) -> Option<&mut dyn GenericTable> {
        match self.object_tables.get_mut(&table) {
            Some(table) => Some(table.as_mut()),
            None => None,
        }
    }

    pub fn get_object_table_specific<SpecificTable: 'static>(&self, table: TableUID) -> Option<&SpecificTable> {
        self.get_object_table(table)?.as_any().downcast_ref()
    }

    pub fn get_object_table_specific_mut<SpecificTable: 'static>(
        &mut self,
        table: TableUID,
    ) -> Option<&mut SpecificTable> {
        self.get_object_table_mut(table)?.as_any_mut().downcast_mut()
    }

    pub fn get_byte_table(&self, table: TableUID) -> Option<&ByteTable> {
        self.byte_tables.get(&table)
    }

    pub fn get_byte_table_mut(&mut self, table: TableUID) -> Option<&mut ByteTable> {
        self.byte_tables.get_mut(&table)
    }

    pub fn authenticate(&self, authority_ref: AuthorityRef, proof: Option<Bytes>) -> Result<BoolOrBytes, MethodStatus> {
        let table_c_pin: &CPINTable =
            self.get_object_table_specific(table_id::C_PIN).ok_or(MethodStatus::TPerMalfunction)?;
        let table_auth: &AuthorityTable =
            self.get_object_table_specific(table_id::AUTHORITY).ok_or(MethodStatus::TPerMalfunction)?;

        let Some(authority) = table_auth.get(&authority_ref) else {
            return Err(MethodStatus::InvalidParameter);
        };
        let credential_ref = authority.credential;
        if credential_ref.is_null() {
            return Ok(BoolOrBytes::Bool(true));
        };
        if let Ok(c_pin_id) = CPINRef::try_new_other(credential_ref) {
            if let Some(credential) = table_c_pin.get(&c_pin_id) {
                let empty_provided_password = vec![];
                let provided_password = proof.as_ref().unwrap_or(&empty_provided_password);
                let success = provided_password == credential.pin.as_slice();
                Ok(BoolOrBytes::Bool(success))
            } else {
                Err(MethodStatus::TPerMalfunction)
            }
        } else {
            Err(MethodStatus::TPerMalfunction)
        }
    }

    pub fn gen_key(
        &mut self,
        credential_ref: CredentialRef,
        _public_exponent: Option<u64>,
        _pin_length: Option<u16>,
    ) -> Result<(), MethodStatus> {
        let k_aes_256_table: &mut KAES256Table =
            self.get_object_table_specific_mut(table_id::K_AES_256).ok_or(MethodStatus::NotAuthorized)?;

        if let Ok(k_aes_256_id) = KAES256Ref::try_new_other(credential_ref) {
            if let Some(object) = k_aes_256_table.get_mut(&k_aes_256_id) {
                object.key = Key256::Bytes64([0xFF; 64]);
                Ok(())
            } else {
                Err(MethodStatus::InvalidParameter)
            }
        } else {
            Err(MethodStatus::InvalidParameter)
        }
    }

    pub fn get_acl(&self, invoking_id: UID, method_id: MethodRef) -> Result<Vec<ACERef>, MethodStatus> {
        let direct_acl = self.access_control.get(&invoking_id, &method_id);
        let table_acl =
            invoking_id.containing_table().map(|table| self.access_control.get(&table, &method_id)).flatten();
        if direct_acl.is_none() && table_acl.is_none() {
            Err(MethodStatus::InvalidParameter)
        } else {
            let mut merged_acl = Vec::new();
            direct_acl.map(|acl| acl.acl.iter().cloned().for_each(|ace_ref| merged_acl.push(ace_ref)));
            table_acl.map(|acl| acl.acl.iter().cloned().for_each(|ace_ref| merged_acl.push(ace_ref)));
            Ok(merged_acl)
        }
    }

    pub fn get(&self, invoking_id: UID, cell_block: CellBlock) -> Result<BytesOrRowValues, MethodStatus> {
        let Some(table_ref) = cell_block.get_target_table(invoking_id) else {
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

    pub fn set(
        &mut self,
        invoking_id: UID,
        where_: Option<u64>,
        values: Option<BytesOrRowValues>,
    ) -> Result<(), MethodStatus> {
        match CellBlockWrite::try_new(invoking_id, where_, values) {
            Ok(CellBlockWrite::Object { table, object, values }) => {
                let table = self.get_object_table_mut(table).ok_or(MethodStatus::InvalidParameter)?;
                let object = table.get_object_mut(object).ok_or(MethodStatus::InvalidParameter)?;

                // Convert values to result(old, new) by try-updating the cells.
                let replaced: Vec<_> = values
                    .into_iter()
                    .map(|(column, content)| (column, object.try_replace(column as usize, content)))
                    .collect();
                // If any cells failed...
                if replaced.iter().any(|(_column, result)| result.is_err()) {
                    // Unroll the cells that succeeded, i.e. result(Ok)
                    replaced.into_iter().for_each(|(column, result)| {
                        let _ = result.map(|old_content| {
                            let _ =
                                object.try_replace(column as usize, old_content).expect("error in From<Value> impl");
                        });
                    });
                    Err(MethodStatus::InvalidParameter)
                } else {
                    Ok(())
                }
            }
            Ok(CellBlockWrite::Byte { table, start_byte, bytes }) => {
                let table = self.get_byte_table_mut(table).ok_or(MethodStatus::InvalidParameter)?;
                table.write(start_byte as usize, bytes.as_slice())
            }
            Ok(CellBlockWrite::None) => Ok(()),
            Err(_) => Err(MethodStatus::InvalidParameter),
        }
    }

    pub fn next(&self, table: TableUID, from: Option<UID>, count: Option<u64>) -> Result<List<UID>, MethodStatus> {
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

    pub fn is_authorized(
        &self,
        authorities: &[AuthorityRef],
        invoking_id: UID,
        method_id: MethodRef,
        columns: &[u16],
    ) -> bool {
        let Ok(acl) = self.get_acl(invoking_id, method_id) else {
            return false;
        };
        let aces = acl
            .iter()
            .map(|ace_ref| self.get_ace(*ace_ref))
            .collect::<Option<Vec<_>>>()
            .expect("invalid device configuration: dangling ACERef");
        is_authorized(authorities, &aces, columns)
    }

    fn get_ace(&self, ace_ref: ACERef) -> Option<&ACE> {
        let ace_table: &ACETable = self.get_object_table_specific(table_id::ACE)?;
        ace_table.get(&ace_ref)
    }
}

fn is_authorized(authorities: &[AuthorityRef], aces: &[&ACE], columns: &[u16]) -> bool {
    let mut authorized_columns = HashSet::new();
    let mut all_columns = false;
    for ace in aces {
        let ace_expr = &ace.boolean_expr;
        let is_authorized = ace_expr.eval(authorities).expect("invalid device configuration: ACE expression invalid");
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
