//! Build script for debshrew
//!
//! This script ensures that the debshrew-minimal WASM module is built
//! before running tests.

use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    // Only build WASM for tests
    if env::var("CARGO_CFG_TEST").is_ok() || env::var("DEBSHREW_BUILD_WASM").is_ok() {
        build_debshrew_minimal_wasm();
    }
    
    // Tell cargo to rerun this script if the debshrew-minimal source changes
    println!("cargo:rerun-if-changed=../debshrew-minimal/src");
    println!("cargo:rerun-if-changed=../debshrew-minimal/Cargo.toml");
}

fn build_debshrew_minimal_wasm() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_dir = Path::new(&manifest_dir).parent().unwrap();
    let debshrew_minimal_dir = workspace_dir.join("debshrew-minimal");
    
    println!("cargo:warning=Building debshrew-minimal WASM module...");
    
    // Build the WASM module
    let output = Command::new("cargo")
        .args(&[
            "build",
            "--target", "wasm32-unknown-unknown",
            "--release",
            "--manifest-path", &debshrew_minimal_dir.join("Cargo.toml").to_string_lossy(),
        ])
        .output()
        .expect("Failed to execute cargo build for debshrew-minimal");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Failed to build debshrew-minimal WASM: {}", stderr);
    }
    
    // Set environment variable for tests
    let wasm_path = workspace_dir
        .join("target/wasm32-unknown-unknown/release/debshrew_minimal.wasm");
    
    println!("cargo:rustc-env=DEBSHREW_MINIMAL_WASM={}", wasm_path.display());
    println!("cargo:warning=debshrew-minimal WASM built successfully at {}", wasm_path.display());
}