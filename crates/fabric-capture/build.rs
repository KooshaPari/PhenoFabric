// build.rs for tf-win-capture
//
// No special build steps needed; windows-sys handles linking via its own build scripts.
// This file exists so `cargo build` works and we can add Windows-specific
// linker flags in the future if needed.

fn main() {
    println!("cargo:rerun-if-changed=src/");
}
