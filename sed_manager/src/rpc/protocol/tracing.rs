//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::{
    rpc::{Error, PackagedMethod},
    spec::ObjectLookup as _,
};

pub fn trace_method(result: &PackagedMethod, direction: &str) {
    use crate::spec::core::OBJECT_LOOKUP;
    match result {
        PackagedMethod::Call(call) => {
            let invoking_id = OBJECT_LOOKUP.by_uid(call.invoking_id, None).unwrap_or(call.invoking_id.to_string());
            let method_id = OBJECT_LOOKUP.by_uid(call.method_id, None).unwrap_or(call.method_id.to_string());
            tracing::event!(
                tracing::Level::DEBUG,
                method_id = method_id,
                invoking_id = invoking_id,
                nargs = call.args.len(),
                status = call.status.to_string(),
                "[{direction}] CALL"
            );
        }
        PackagedMethod::Result(result) => {
            tracing::event!(
                tracing::Level::DEBUG,
                nargs = result.results.len(),
                status = result.status.to_string(),
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
