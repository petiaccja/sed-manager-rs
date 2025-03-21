//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

pub mod applications;
pub mod device;
pub mod fake_device;
pub mod messaging;
pub mod rpc;
pub mod serialization;
pub mod spec;
pub mod tper;

mod call_with_tuple;
mod variadics;
mod with_copy;

extern crate self as sed_manager;
