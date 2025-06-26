use debshrew::client::MockMetashrewClient;
use debshrew::sink::ConsoleSink;
use debshrew::WasmRuntime;
use debshrew::synchronizer::BlockSynchronizer;
use std::path::Path;
use alkanes_support::proto::alkanes::BlockResponse;
use protobuf::Message;

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
    
    // Load the sample transform WASM module
    let wasm_path = Path::new("./target/wasm32-unknown-unknown/release/sample_transform.wasm");
    println!("Loading WASM module from {:?}", wasm_path);
    let mut runtime = WasmRuntime::new(wasm_path, "http://localhost:8080")?;
    
    // Register a mock "getblock" view function
    println!("Registering mock getblock view function");
    runtime.register_view_function(
        "getblock",
        Box::new(|_params| {
            // Create a minimal valid Bitcoin block bytes
            // This is a simplified version that just contains enough data to be parsed
            // by the consensus_decode function in the sample-transform
            let block_bytes = vec![
                // Version (4 bytes)
                1, 0, 0, 0,
                // Previous block hash (32 bytes)
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                // Merkle root (32 bytes)
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                // Timestamp (4 bytes) - 1623456789 in little-endian
                0x15, 0x5B, 0xD0, 0x60,
                // Bits (4 bytes)
                0xFF, 0xFF, 0x00, 0x1D,
                // Nonce (4 bytes)
                0, 0, 0, 0,
                // Tx count (1 byte) - VarInt for 0 transactions
                0
            ];
            
            // Create a BlockResponse protobuf message
            let mut response = BlockResponse::new();
            response.block = block_bytes;
            
            // Serialize the response to bytes
            match response.write_to_bytes() {
                Ok(bytes) => Ok(bytes),
                Err(e) => Err(format!("Failed to serialize BlockResponse: {}", e).into())
            }
        })
    );
    
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