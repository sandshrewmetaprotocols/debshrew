use anyhow::{anyhow, Result};
use bitcoin::Block;
use reqwest::blocking::Client;
use serde_json::{json, Value};
use alkanes_support::proto::alkanes::{BlockRequest, BlockResponse};
use protobuf::Message;
use std::env;

fn main() -> Result<()> {
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
    
    // Block height to request - use command line arg if provided, otherwise default
    let height = if args.len() > 2 {
        args[2].parse::<u32>().unwrap_or(1)
    } else {
        1 // Start with block 1
    };
    
    println!("Requesting block {} from {}", height, url);
    
    // Create a BlockRequest using the protobuf type
    let mut block_request = BlockRequest::new();
    block_request.height = height;
    
    // Serialize the BlockRequest to bytes using protobuf
    let request_bytes = block_request.write_to_bytes()?;
    
    // Encode the input data as hex for the JSON-RPC request
    let hex_input = hex::encode(&request_bytes);
    
    // Create the JSON-RPC request
    let json_rpc_request = json!({
        "jsonrpc": "2.0",
        "method": "metashrew_view",
        "params": ["getblock", hex_input, "latest"],
        "id": 1
    });
    
    println!("Sending JSON-RPC request: {}", json_rpc_request);
    
    // Send the request
    let response = client.post(url)
        .header("Content-Type", "application/json")
        .body(json_rpc_request.to_string())
        .send()?;
    
    // Get the response text
    let response_text = response.text()?;
    println!("Received response: {}", response_text);
    
    // Parse the response
    let response_json: Value = serde_json::from_str(&response_text)?;
    
    // Check for errors
    if let Some(error) = response_json.get("error") {
        return Err(anyhow!("JSON-RPC error: {:?}", error));
    }
    
    // Get the result
    let result = response_json["result"].as_str().ok_or_else(|| anyhow!("Invalid result"))?;
    
    // Strip the "0x" prefix if present
    let clean_result = if result.starts_with("0x") {
        &result[2..]
    } else {
        result
    };
    
    // Decode the hex result
    let result_bytes = hex::decode(clean_result)?;
    
    // Parse the BlockResponse using protobuf
    let block_response = BlockResponse::parse_from_bytes(&result_bytes)?;
    
    println!("Block height: {}", block_response.height);
    println!("Block data size: {} bytes", block_response.block.len());
    
    // Decode the block
    let block = bitcoin::consensus::encode::deserialize::<Block>(&block_response.block)?;
    
    // Print block information
    println!("Block version: {:?}", block.header.version);
    println!("Block time: {}", block.header.time);
    println!("Block merkle root: {}", block.header.merkle_root);
    println!("Number of transactions: {}", block.txdata.len());
    
    Ok(())
}