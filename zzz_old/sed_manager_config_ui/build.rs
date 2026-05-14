//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

fn main() {
    slint_build::compile("ui/app_window.slint").expect("Slint build failed");
    println!("cargo::rerun-if-changed=ui/")
}
