use reqwest::Client;
use serde_json::{json, Value};
use std::error::Error;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create a new HTTP client
    let client = Client::new();
    
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    
    // Metashrew URL - use command line arg if provided, otherwise default
    let url = if args.len() > 1 {
        args[1].clone()
    } else {
        "http://localhost:18888".to_string()
    };
    
    println!("Testing Metashrew API at {}", url);
    
    // Test 1: Get height
    println!("\n=== Testing metashrew_height ===");
    let height_response = client.post(&url)
        .header("Content-Type", "application/json")
        .body(json!({
            "jsonrpc": "2.0",
            "method": "metashrew_height",
            "params": [],
            "id": 1
        }).to_string())
        .send()
        .await?;
    
    let height_text = height_response.text().await?;
    println!("Response: {}", height_text);
    
    // Try to parse the height
    let height_json: Value = serde_json::from_str(&height_text)?;
    let height = if let Some(result) = height_json.get("result") {
        if let Some(height_str) = result.as_str() {
            height_str.parse::<u32>().unwrap_or(0)
        } else {
            result.as_u64().unwrap_or(0) as u32
        }
    } else {
        println!("No result field in response");
        0
    };
    
    println!("Parsed height: {}", height);
    
    // Test 2: Get block hash for height 2 (should fail if there's only 1 block)
    println!("\n=== Testing metashrew_getblockhash for height 2 ===");
    let blockhash_response = client.post(&url)
        .header("Content-Type", "application/json")
        .body(json!({
            "jsonrpc": "2.0",
            "method": "metashrew_getblockhash",
            "params": [2],
            "id": 2
        }).to_string())
        .send()
        .await?;
    
    let blockhash_text = blockhash_response.text().await?;
    println!("Response: {}", blockhash_text);
    
    // Test 3: Try with bitcoin-style method
    println!("\n=== Testing getblockhash for height 2 (bitcoin-style) ===");
    let bitcoin_blockhash_response = client.post(&url)
        .header("Content-Type", "application/json")
        .body(json!({
            "jsonrpc": "2.0",
            "method": "getblockhash",
            "params": [2],
            "id": 3
        }).to_string())
        .send()
        .await?;
    
    let bitcoin_blockhash_text = bitcoin_blockhash_response.text().await?;
    println!("Response: {}", bitcoin_blockhash_text);
    
    // Test 4: Check if we can get block info for height 1
    println!("\n=== Testing getblock for height 1 ===");
    
    // First get the hash
    let hash_response = client.post(&url)
        .header("Content-Type", "application/json")
        .body(json!({
            "jsonrpc": "2.0",
            "method": "getblockhash",
            "params": [1],
            "id": 4
        }).to_string())
        .send()
        .await?;
    
    let hash_json: Value = serde_json::from_str(&hash_response.text().await?)?;
    let hash = hash_json.get("result").and_then(|v| v.as_str()).unwrap_or("");
    
    // Then get the block info
    let block_response = client.post(&url)
        .header("Content-Type", "application/json")
        .body(json!({
            "jsonrpc": "2.0",
            "method": "getblock",
            "params": [hash],
            "id": 5
        }).to_string())
        .send()
        .await?;
    
    let block_text = block_response.text().await?;
    println!("Response: {}", block_text);
    
    // Test 5: Check if we can get blockchain info
    println!("\n=== Testing getblockchaininfo ===");
    let info_response = client.post(&url)
        .header("Content-Type", "application/json")
        .body(json!({
            "jsonrpc": "2.0",
            "method": "getblockchaininfo",
            "params": [],
            "id": 6
        }).to_string())
        .send()
        .await?;
    
    let info_text = info_response.text().await?;
    println!("Response: {}", info_text);
    
    // Test 6: Check if we can get the actual block count
    println!("\n=== Testing getblockcount ===");
    let count_response = client.post(&url)
        .header("Content-Type", "application/json")
        .body(json!({
            "jsonrpc": "2.0",
            "method": "getblockcount",
            "params": [],
            "id": 7
        }).to_string())
        .send()
        .await?;
    
    let count_text = count_response.text().await?;
    println!("Response: {}", count_text);
    
    Ok(())
}