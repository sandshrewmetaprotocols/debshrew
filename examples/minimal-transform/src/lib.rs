use debshrew_runtime::{self, DebTransform};
use debshrew_support::{CdcMessage, CdcHeader, CdcOperation, CdcPayload};

#[derive(Default, Clone, Debug)]
pub struct MinimalTransform {
    // State fields (none needed for this simple transform)
}

impl DebTransform for MinimalTransform {
    fn process_block(&mut self) -> debshrew_runtime::error::Result<Vec<CdcMessage>> {
        // Get current block info
        let height = debshrew_runtime::get_height();
        let hash = debshrew_runtime::get_block_hash();
        
        debshrew_runtime::println!("MinimalTransform: Processing block {} with hash {}", height, hex::encode(&hash));
        
        // Create a simple CDC message
        let message = CdcMessage {
            header: CdcHeader {
                source: "minimal_transform".to_string(),
                timestamp: 1623456789000, // Fixed timestamp in milliseconds
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
                    "timestamp": 1623456789 // Fixed timestamp in seconds
                })),
            },
        };
        
        // Return CDC message
        debshrew_runtime::println!("MinimalTransform: Creating CDC message...");
        
        debshrew_runtime::println!("MinimalTransform: Block processing complete");
        
        Ok(vec![message])
    }
}

// Register the transform
debshrew_runtime::declare_transform!(MinimalTransform);