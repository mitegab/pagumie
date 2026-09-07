//! Generates the C header with cbindgen.
//!
//! The header is written to `OUT_DIR` on every build, and additionally to the
//! committed `include/ethiopic.h` when `ETHIOPIC_UPDATE_HEADER=1`. CI sets that
//! variable and then requires `git diff --exit-code`, so the committed header
//! can never drift from the Rust source.

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=cbindgen.toml");
    println!("cargo:rerun-if-env-changed=ETHIOPIC_UPDATE_HEADER");

    let crate_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let Ok(bindings) = cbindgen::generate(&crate_dir) else {
        // Never fail the build of the library itself because of the header
        // generator; the CI job that checks the header will catch problems.
        println!("cargo:warning=cbindgen failed; skipping header generation");
        return;
    };

    bindings.write_to_file(out_dir.join("ethiopic.h"));
    if std::env::var_os("ETHIOPIC_UPDATE_HEADER").is_some() {
        bindings.write_to_file(crate_dir.join("include").join("ethiopic.h"));
    }
}
