use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../frontend/src");
    println!("cargo:rerun-if-changed=../frontend/package.json");
    println!("cargo:rerun-if-changed=../frontend/vite.config.ts");
    println!("cargo:rerun-if-changed=../frontend/svelte.config.js");

    let status = Command::new("npm")
        .args(["run", "build"])
        .current_dir("../frontend")
        .status()
        .expect("Failed to build frontend");

    if !status.success() {
        panic!("Frontend build failed");
    }
}
