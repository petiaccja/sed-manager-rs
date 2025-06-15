//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::messaging::value::{Named, Value};
use crate::rpc::{Error, PackagedMethod};
use crate::spec::ObjectLookup as _;

pub fn trace_method(result: &PackagedMethod, direction: &str) {
    use crate::spec::core::OBJECT_LOOKUP;
    match result {
        PackagedMethod::Call(call) => {
            let mut invoking_id = OBJECT_LOOKUP.by_uid(call.invoking_id, None).unwrap_or(call.invoking_id.to_string());
            let method_id = OBJECT_LOOKUP.by_uid(call.method_id, None).unwrap_or(call.method_id.to_string());
            if let Some(table) = call.invoking_id.containing_table() {
                let table = OBJECT_LOOKUP.by_uid(table, None).unwrap_or(table.to_string());
                invoking_id = format!("{table}::{invoking_id}")
            }
            let args = format!("{:?}", sanitize(Value::from(call.args.clone())));
            tracing::event!(
                tracing::Level::DEBUG,
                method_id = method_id,
                invoking_id = invoking_id,
                status = call.status.to_string(),
                args = args,
                "[{direction}] CALL"
            );
        }
        PackagedMethod::Result(result) => {
            let results = format!("{:?}", sanitize(Value::from(result.results.clone())));
            tracing::event!(
                tracing::Level::DEBUG,
                status = result.status.to_string(),
                results = results,
                "[{direction}] RESULT"
            );
        }
        PackagedMethod::EndOfSession => tracing::event!(tracing::Level::DEBUG, "[{direction}] EOS"),
    }
}

pub fn trace_maybe_method(result: &Result<PackagedMethod, Error>, direction: &str) {
    match result {
        Ok(method) => trace_method(method, direction),
        Err(error) => tracing::event!(tracing::Level::DEBUG, message = error.to_string(), "[{direction}] ERROR"),
    }
}

/// Remove sensitive information from values so that they are not present in log files.
///
/// The most sensitive information is passwords, but sensitive information can
/// also be uploaded to the DataStore and MBR tables.
/// Luckily, all sensitive information is stored as bytes, so we can just redact
/// all byte data from [`Value`]s.
fn sanitize(value: Value) -> Value {
    match value {
        Value::Empty => value,
        Value::Int8(_) => value,
        Value::Int16(_) => value,
        Value::Int32(_) => value,
        Value::Int64(_) => value,
        Value::Uint8(_) => value,
        Value::Uint16(_) => value,
        Value::Uint32(_) => value,
        Value::Uint64(_) => value,
        Value::Command(_) => value,
        Value::Named(named) => Value::from(Named { name: named.name, value: sanitize(named.value) }),
        Value::Bytes(_) => Value::from(Vec::<u8>::new()),
        Value::List(list) => Value::from(list.into_iter().map(|v| sanitize(v)).collect::<Vec<_>>()),
    }
}
