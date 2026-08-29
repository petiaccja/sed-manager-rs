use std::path::Path;

fn rerun_if_slint_changed(dir: &Path) {
    for entry in std::fs::read_dir(dir).expect("failed to read ui directory") {
        let entry = entry.expect("failed to read directory entry");
        let path = entry.path();
        if path.is_dir() {
            rerun_if_slint_changed(&path);
        } else if path.extension().map_or(false, |ext| ext == "slint") {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}

fn main() {
    rerun_if_slint_changed(Path::new("ui"));

    // Slint debug info is needed by `i-slint-backend-testing`'s `ElementHandle` API.
    // Add it ONLY in debug builds.
    let debug_info = std::env::var("PROFILE").as_deref() == Ok("debug");
    let config = slint_build::CompilerConfiguration::new().with_debug_info(debug_info);
    slint_build::compile_with_config("ui/main_window.slint", config).expect("Slint build failed");
}
