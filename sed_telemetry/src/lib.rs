//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

//! Utilities related to tracing and exporting traces when the app is running.

pub mod otlp;
mod with_tracing;

pub use sed_telemetry_macros::with_tracing;

pub mod macro_support {
    pub use super::with_tracing::with_tracing;
}

extern crate self as sed_telemetry;
