//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::collections::HashSet;
use std::sync::LazyLock;

use crate::messaging::uid::UID;
use crate::messaging::value::{Named, Value};
use crate::rpc::{Error, PackagedMethod};
use crate::spec::{method_id, table_id};

pub fn trace_method(result: &PackagedMethod, direction: &str) {
    match result {
        PackagedMethod::Call(call) => {
            let args = sanitize(Value::from(call.args.clone()), call.invoking_id, call.method_id);
            tracing::event!(
                tracing::Level::DEBUG,
                method_id = call.method_id.to_string(),
                invoking_id = call.invoking_id.to_string(),
                status = call.status.to_string(),
                args = to_trace_json(&args),
                "[{direction}] CALL"
            );
        }
        PackagedMethod::Result(result) => {
            let results = Value::from(result.results.clone());
            tracing::event!(
                tracing::Level::DEBUG,
                status = result.status.to_string(),
                results = to_trace_json(&results),
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

static NON_SENSITIVE_METHODS: LazyLock<HashSet<UID>> = LazyLock::new(|| {
    [
        method_id::ACTIVATE.as_uid(),
        method_id::ADD_ACE.as_uid(),
        method_id::ERASE.as_uid(),
        method_id::GEN_KEY.as_uid(),
        method_id::GET_ACL.as_uid(),
        method_id::NEXT.as_uid(),
        method_id::REACTIVATE.as_uid(),
        method_id::REMOVE_ACE.as_uid(),
        method_id::REVERT.as_uid(),
        method_id::REVERT_SP.as_uid(),
    ]
    .into_iter()
    .collect()
});

static NON_SENSITIVE_TABLES: LazyLock<HashSet<UID>> = LazyLock::new(|| {
    [
        table_id::ACCESS_CONTROL.into(),
        table_id::ACE.into(),
        table_id::AUTHORITY.into(),
        table_id::LOCKING.into(),
        table_id::LOCKING_INFO.into(),
        table_id::MBR_CONTROL.into(),
        table_id::SP.into(),
        table_id::SP_INFO.into(),
        table_id::TABLE.into(),
        table_id::T_PER_INFO.into(),
    ]
    .into_iter()
    .collect()
});

/// Remove sensitive information from values so that they are not present in log files.
///
/// The most sensitive information is passwords, but sensitive information can
/// also be uploaded to the DataStore and MBR tables.
/// Luckily, all sensitive information is stored as bytes, so we can just redact
/// all byte data from [`Value`]s.
fn sanitize(value: Value, invoking_id: UID, method_id: UID) -> Value {
    if NON_SENSITIVE_METHODS.contains(&method_id) {
        return value;
    }
    if invoking_id.containing_table().is_some_and(|table_id| NON_SENSITIVE_TABLES.contains(&table_id)) {
        return value;
    }
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
        Value::Named(named) => {
            Value::from(Named { name: named.name, value: sanitize(named.value, invoking_id, method_id) })
        }
        Value::Bytes(_) => Value::from(Vec::from(b"REDACTED")),
        Value::List(list) => {
            Value::from(list.into_iter().map(|v| sanitize(v, invoking_id, method_id)).collect::<Vec<_>>())
        }
    }
}

fn to_trace_json(value: &Value) -> String {
    match value {
        Value::Empty => "null".into(),
        Value::Int8(n) => format!("{{ \"i8\": {n}}}"),
        Value::Int16(n) => format!("{{ \"i16\": {n}}}"),
        Value::Int32(n) => format!("{{ \"i32\": {n}}}"),
        Value::Int64(n) => format!("{{ \"i64\": {n}}}"),
        Value::Uint8(n) => format!("{{ \"u8\": {n}}}"),
        Value::Uint16(n) => format!("{{ \"u16\": {n}}}"),
        Value::Uint32(n) => format!("{{ \"u32\": {n}}}"),
        Value::Uint64(n) => format!("{{ \"u64\": {n}}}"),
        Value::Command(command) => format!("{{ \"command\": \"{command:?}\"}}"),
        Value::Named(named) => {
            format!(
                "{{ \"named\": {{ \"name\": {}, \"value\": {} }} }}",
                to_trace_json(&named.name),
                to_trace_json(&named.value)
            )
        }
        Value::Bytes(items) => {
            format!("{{ \"bytes\": [ {} ] }}", items.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(", "))
        }
        Value::List(values) => {
            format!("{{ \"list\": [ {} ] }}", values.iter().map(|v| to_trace_json(v)).collect::<Vec<_>>().join(", "))
        }
    }
}
