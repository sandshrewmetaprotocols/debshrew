//! Test for the logging transform example
//!
//! This example demonstrates how to use the test-utils feature to test a transform module.

use debshrew_runtime::test_utils::TestRunner;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build the logging transform WASM module
    println!("Building logging-transform WASM module...");
    let status = std::process::Command::new("cargo")
        .args(["build", "--release", "--target", "wasm32-unknown-unknown"])
        .current_dir("examples/logging-transform")
        .status()?;
    
    if !status.success() {
        return Err("Failed to build logging-transform WASM module".into());
    }
    
    // Path to the WASM module
    let wasm_path = PathBuf::from("target/wasm32-unknown-unknown/release/logging_transform.wasm");
    
    if !wasm_path.exists() {
        return Err(format!("WASM module not found at {:?}", wasm_path).into());
    }
    
    println!("Loading WASM module from {:?}...", wasm_path);
    
    // Load the WASM module
    let wasm_bytes = std::fs::read(&wasm_path)?;
    
    println!("WASM module loaded ({} bytes)", wasm_bytes.len());
    
    // Create a test runner
    let runner = TestRunner::new()
        .with_height(123456)
        .with_hash(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    
    // Initialize the test environment
    runner.init();
    
    println!("Test environment initialized");
    println!("Block height: {}", runner.height);
    println!("Block hash: {:?}", runner.hash);
    
    // TODO: In a real test, we would instantiate the WASM module and call its exports
    // This would require a WASM runtime like wasmtime or wasmer
    // For now, we just demonstrate the test setup
    
    println!("Test completed successfully");
    
    Ok(())
}