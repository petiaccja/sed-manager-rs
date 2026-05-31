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
    slint_build::compile("ui/main_window.slint").expect("Slint build failed");
}
