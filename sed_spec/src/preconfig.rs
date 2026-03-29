#![allow(unused)]

include!(concat!(env!("OUT_DIR"), "/spec.rs"));

/// The purpose of this is only so that it's easy to jump into the generated
/// code using "go to definition".
#[allow(unused)]
const MARKER: () = GENERATED_MARKER;
