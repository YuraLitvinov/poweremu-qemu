//! Generates the Rust view of QEMU's C interface from its headers.

use std::env;
use std::path::PathBuf;

fn main() {
    let dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let display = dir.join("..");
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed={}", display.join("ppc_mac_gpu_renderer.h").display());
    println!("cargo:rerun-if-changed={}", display.join("r300").display());

    let bindings = bindgen::Builder::default()
        .header(dir.join("wrapper.h").to_string_lossy().into_owned())
        .clang_arg(format!("-I{}", display.display()))
        .allowlist_type("PPCMacGPU.*")
        .allowlist_type("R300.*")
        .allowlist_type("R200.*")
        .allowlist_function("r300_.*")
        .allowlist_var("R300_.*")
        .allowlist_var("PPC_MAC_GPU_.*")
        .derive_default(false)
        .generate()
        .expect("bindgen failed on the QEMU headers");
    bindings
        .write_to_file(PathBuf::from(env::var("OUT_DIR").unwrap()).join("bindings.rs"))
        .unwrap();
}
