use std::fs::File;
use std::io::{Read as _, Write as _};
use std::path::PathBuf;

fn main() -> Result<(), ()> {
    let spec_path = "spec.json";
    let mut spec_file = File::open(spec_path).unwrap();
    let mut spec_content = String::new();
    spec_file.read_to_string(&mut spec_content).unwrap();

    let generated_spec = sed_spec_codegen::generate_spec(&spec_content).unwrap();

    let mut out_path = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    out_path.push("spec.rs");
    let mut out_file = File::create(out_path).unwrap();
    out_file.write_all(generated_spec.as_bytes()).unwrap();
    println!("cargo::rerun-if-changed={spec_path}");
    Ok(())
}
