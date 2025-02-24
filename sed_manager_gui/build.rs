fn main() {
    slint_build::compile("ui/app_window.slint").expect("Slint build failed");
    println!("cargo::rerun-if-changed=ui/")
}
