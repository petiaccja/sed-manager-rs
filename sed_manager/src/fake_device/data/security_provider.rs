use crate::messaging::types::{AuthorityRef, BoolOrBytes, Password, SPRef};
use crate::messaging::uid::UID;
use crate::messaging::value::{Bytes, Value};
use crate::rpc::MethodStatus;
use crate::specification::table;

use super::objects::{AuthorityTable, CPinTable};
use super::table::BasicTable;

pub trait SecurityProvider {
    fn uid(&self) -> SPRef;
    fn get_table(&self, uid: UID) -> Option<&dyn BasicTable>;
    fn get_table_mut(&mut self, uid: UID) -> Option<&mut dyn BasicTable>;
    fn get_c_pin_table(&self) -> Option<&CPinTable>;
    fn get_authority_table(&self) -> Option<&AuthorityTable>;
}

pub fn authenticate(
    sp: &dyn SecurityProvider,
    authority: AuthorityRef,
    proof: Option<Bytes>,
) -> Result<BoolOrBytes, MethodStatus> {
    let Some(authority_table) = sp.get_authority_table() else {
        return Err(MethodStatus::InvalidParameter);
    };
    let Some(authority_obj) = authority_table.0.get(&authority) else {
        return Err(MethodStatus::InvalidParameter);
    };
    let Some(credential_ref) = authority_obj.credential else {
        return Ok(BoolOrBytes::Bool(true));
    };
    if credential_ref.containing_table().unwrap() == table::C_PIN {
        let Some(c_pin_table) = sp.get_c_pin_table() else {
            return Err(MethodStatus::TPerMalfunction);
        };
        if let Some(credential_obj) = c_pin_table.0.get(&credential_ref) {
            let pw_correct = credential_obj.pin.as_ref().unwrap_or(&Password::default()).0 == proof.unwrap_or(vec![]);
            Ok(BoolOrBytes::Bool(pw_correct))
        } else {
            Err(MethodStatus::TPerMalfunction)
        }
    } else {
        Err(MethodStatus::TPerMalfunction)
    }
}

pub fn get(sp: &dyn SecurityProvider, object: UID, column: usize) -> Result<Value, MethodStatus> {
    let Some(table) = object.containing_table() else {
        return Err(MethodStatus::InvalidParameter);
    };
    let Some(table) = sp.get_table(table) else {
        return Err(MethodStatus::InvalidParameter);
    };
    let Some(object) = table.get_object(object) else {
        return Err(MethodStatus::InvalidParameter);
    };
    Ok(object.get_column(column))
}

pub fn set(sp: &mut dyn SecurityProvider, object: UID, column: usize, value: Value) -> Result<(), MethodStatus> {
    let Some(table) = object.containing_table() else {
        return Err(MethodStatus::InvalidParameter);
    };
    let Some(table) = sp.get_table_mut(table) else {
        return Err(MethodStatus::InvalidParameter);
    };
    let Some(object) = table.get_object_mut(object) else {
        return Err(MethodStatus::InvalidParameter);
    };
    match object.try_set_column(column, value) {
        Ok(_) => Ok(()),
        Err(_) => Err(MethodStatus::InvalidParameter),
    }
}
