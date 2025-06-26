use debshrew::WasmRuntime;
use std::path::Path;
use alkanes_support::proto::alkanes::BlockResponse;
use protobuf::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the sample transform WASM module
    let wasm_path = Path::new("./target/wasm32-unknown-unknown/release/sample_transform.wasm");
    println!("Loading WASM module from {:?}", wasm_path);
    let mut runtime = WasmRuntime::new(wasm_path, "http://localhost:8080")?;
    
    // Register a mock "getblock" view function
    println!("Registering mock getblock view function");
    runtime.register_view_function(
        "getblock",
        Box::new(|_params| {
            println!("Mock getblock view function called");
            
            // Create a minimal valid Bitcoin block bytes
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
                Ok(bytes) => {
                    println!("Mock getblock view function returning {} bytes", bytes.len());
                    Ok(bytes)
                },
                Err(e) => Err(format!("Failed to serialize BlockResponse: {}", e).into())
            }
        })
    );
    
    // Set up the runtime with test data
    runtime.set_current_height(10);
    runtime.set_current_hash(vec![10 as u8; 32]); // 32-byte hash
    
    // Process a block
    println!("Processing block...");
    match runtime.process_block(10, vec![10 as u8; 32]) {
        Ok(_) => println!("Block processed successfully"),
        Err(e) => println!("Error processing block: {:?}", e),
    }
    
    println!("Test completed successfully");
    Ok(())
}