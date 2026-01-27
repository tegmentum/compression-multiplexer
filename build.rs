use std::env;
use std::path::PathBuf;

fn main() {
    let target = env::var("TARGET").unwrap();
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    // Link OpenZL libraries for WASM targets
    if target.contains("wasm") {
        let openzl_libs = manifest_dir.join("openzl-libs");

        if openzl_libs.join("libopenzl.a").exists() {
            println!(
                "cargo:rustc-link-search=native={}",
                openzl_libs.display()
            );
            println!("cargo:rustc-link-lib=static=openzl");
            println!("cargo:rustc-link-lib=static=zstd");
        } else {
            println!("cargo:warning=OpenZL libraries not found at {:?}", openzl_libs);
            println!("cargo:warning=OpenZL compression will not be available");
        }
    }

    println!("cargo:rerun-if-changed=openzl-libs");
    println!("cargo:rerun-if-changed=build.rs");
}
