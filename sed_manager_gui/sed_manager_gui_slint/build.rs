//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::path::Path;

fn rerun_if_slint_changed(dir: &Path) {
    for entry in std::fs::read_dir(dir).expect("failed to read ui directory") {
        let entry = entry.expect("failed to read directory entry");
        let path = entry.path();
        if path.is_dir() {
            rerun_if_slint_changed(&path);
        } else if path.extension().is_some_and(|ext| ext == "slint") {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}

fn main() {
    rerun_if_slint_changed(Path::new("ui"));

    // Slint debug info is needed by `i-slint-backend-testing`'s `ElementHandle` API.
    // It would be better to add this only in test builds, but it's needed for both
    // debug and release.
    let config = slint_build::CompilerConfiguration::new().with_debug_info(true);
    slint_build::compile_with_config("ui/main_window.slint", config).expect("Slint build failed");
}
