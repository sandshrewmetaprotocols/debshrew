// Sample transform that doesn't use wasm-bindgen
// Based on the minimal transform example

use debshrew_runtime::{self, DebTransform};
use debshrew_runtime::{CdcMessage, CdcHeader, CdcOperation, CdcPayload};
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug)]
pub struct BlockTransform {
    // State fields (none needed for this simple transform)
}

// Request structure for the getblock view function
#[derive(Serialize, Deserialize)]
struct BlockRequest {
    height: u32,
}

impl DebTransform for BlockTransform {
    fn process_block(&mut self) -> debshrew_runtime::Result<()> {
        // Get current block info
        let height = debshrew_runtime::get_height();
        let hash = debshrew_runtime::get_block_hash();
        
        debshrew_runtime::println!("Processing block {} with hash {}", height, hex::encode(&hash));
        
        // Create a request for the getblock view function
        let params = debshrew_runtime::serialize_params(&BlockRequest { 
            height: height 
        })?;
        
        debshrew_runtime::println!("Fetching block data from alkanes-rs...");
        
        // Call the getblock view function
        let result = match debshrew_runtime::view("getblock".to_string(), params) {
            Ok(data) => data,
            Err(e) => {
                debshrew_runtime::eprintln!("Error calling getblock view: {:?}", e);
                return Err(e);
            }
        };
        
        debshrew_runtime::println!("Received block data of size: {} bytes", result.len());
        
        // Create a CDC message with the block data
        let message = CdcMessage {
            header: CdcHeader {
                source: "block_transform".to_string(),
                timestamp: chrono::Utc::now(),
                block_height: height,
                block_hash: hex::encode(&hash),
                transaction_id: None,
            },
            payload: CdcPayload {
                operation: CdcOperation::Create,
                table: "blocks".to_string(),
                key: height.to_string(),
                before: None,
                after: Some(serde_json::json!({
                    "height": height,
                    "hash": hex::encode(&hash),
                    "raw_data": hex::encode(&result),
                    "timestamp": chrono::Utc::now()
                })),
            },
        };
        
        // Push CDC message to Kafka
        debshrew_runtime::println!("Pushing CDC message to Kafka...");
        self.push_message(message)?;
        
        debshrew_runtime::println!("Block processing complete");
        
        Ok(())
    }
    
    // We don't need to implement rollback() as the default implementation
    // will use the automatically generated inverse operations
}

// Register the transform
debshrew_runtime::declare_transform!(BlockTransform);
