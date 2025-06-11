use debshrew::client::MockMetashrewClient;
use debshrew::sink::ConsoleSink;
use debshrew::WasmRuntime;
use debshrew::synchronizer::BlockSynchronizer;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a mock metashrew client
    let mut client = MockMetashrewClient::new();
    
    // Set up the mock client with some test data
    client.set_height(10);
    
    // Add block hashes for heights 0 through 10
    for i in 0..=10 {
        // Create a simple hash (in a real scenario, this would be a proper hash)
        let hash = vec![i as u8; 32]; // 32-byte hash
        client.set_block_hash(i, hash);
    }
    
    // Load the minimal transform WASM module
    let wasm_path = Path::new("./target/wasm32-unknown-unknown/release/minimal_transform.wasm");
    println!("Loading WASM module from {:?}", wasm_path);
    let runtime = WasmRuntime::new(wasm_path)?;
    
    // Create a console sink to output CDC messages
    let sink = Box::new(ConsoleSink::new(true)); // true for pretty printing
    
    // Create a block synchronizer with a cache size of 6
    let mut synchronizer = BlockSynchronizer::new(client, runtime, sink, 6)?;
    
    // Set a short polling interval (100ms)
    synchronizer.set_polling_interval(100);
    
    // Run the synchronizer for a short time
    println!("Starting block synchronization");
    let sync_task = tokio::spawn(async move {
        if let Err(e) = synchronizer.run().await {
            eprintln!("Synchronizer error: {}", e);
        }
    });
    
    // Let it run for 2 seconds
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Abort the synchronization task
    sync_task.abort();
    
    println!("Test completed successfully");
    Ok(())
}