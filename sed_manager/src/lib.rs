#![allow(dead_code)]

pub mod device;
pub mod fake_device;
pub mod messaging;
pub mod rpc;
pub mod serialization;
pub mod specification;
pub mod tper;

mod sync;
mod variadics;
mod with_copy;

extern crate self as sed_manager;
