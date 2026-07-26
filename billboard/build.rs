//! Puts the identifier registry where `billboard::registry` can `include!` it.
//!
//! This build script **only copies a file**. It never parses, generates, or
//! validates Rust — the plugin's `/billboard export registry` emits Rust
//! source directly, so `rustc` is the validator and a malformed export fails
//! loudly at compile time with real spans instead of being mangled here.
//!
//! Source of the file:
//! 1. `$BILLBOARD_REGISTRY` — a registry exported from a running server
//!    (the real block/item set of that exact version + its plugins).
//! 2. otherwise `billboard/registry-snapshot.rs` — the bundled stub, enough
//!    of vanilla to build and test the SDK.
//!
//! Rerun rules: the env var (`rerun-if-env-changed`), whichever file we copied
//! (`rerun-if-changed`), and this script itself — emitting any
//! `rerun-if-changed` opts out of Cargo's default "rerun on any package
//! change", so `build.rs` must be listed explicitly.

use std::path::PathBuf;
use std::{env, fs};

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-env-changed=BILLBOARD_REGISTRY");

    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let bundled = manifest.join("registry-snapshot.rs");

    // Watch the bundled snapshot either way: if $BILLBOARD_REGISTRY is unset
    // later, this is the file that becomes live again.
    println!("cargo::rerun-if-changed={}", bundled.display());

    let source = match env::var_os("BILLBOARD_REGISTRY") {
        Some(path) if !path.is_empty() => {
            let path = PathBuf::from(path);
            println!("cargo::rerun-if-changed={}", path.display());
            path
        }
        _ => bundled,
    };

    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR")).join("registry.rs");
    if let Err(e) = fs::copy(&source, &out) {
        panic!(
            "billboard: cannot copy the identifier registry from {} to {}: {e}\n\
             (set BILLBOARD_REGISTRY to a registry.rs exported by \
             `/billboard export registry`, or unset it to use the bundled snapshot)",
            source.display(),
            out.display(),
        );
    }
}
