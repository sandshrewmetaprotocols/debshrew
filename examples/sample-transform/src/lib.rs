use debshrew_runtime::{self, DebTransform};
use debshrew_runtime::{CdcMessage, CdcHeader, CdcOperation, CdcPayload};

#[derive(Default, Clone, Debug)]
pub struct BlockTransform {
    // State fields (none needed for this simple transform)
}

impl DebTransform for BlockTransform {
    fn process_block(&mut self) -> debshrew_runtime::Result<Vec<CdcMessage>> {
        // Get current block info
        let height = debshrew_runtime::get_height();
        let hash = debshrew_runtime::get_block_hash();
        
        debshrew_runtime::println!("BlockTransform: Processing block {} with hash {}", height, hex::encode(&hash));
        
        // Create a simple CDC message
        let message = CdcMessage {
            header: CdcHeader {
                source: "block_transform".to_string(),
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
        
        debshrew_runtime::println!("BlockTransform: Created CDC message");
        
        // Return the CDC messages
        debshrew_runtime::println!("BlockTransform: Block processing complete");
        
        Ok(vec![message])
    }
    
    fn rollback(&mut self) -> debshrew_runtime::Result<Vec<CdcMessage>> {
        // Get current block info
        let height = debshrew_runtime::get_height();
        let hash = debshrew_runtime::get_block_hash();
        
        debshrew_runtime::println!("BlockTransform: Rolling back block {} with hash {}", height, hex::encode(&hash));
        
        // Create a simple CDC message for rollback
        let message = CdcMessage {
            header: CdcHeader {
                source: "block_transform".to_string(),
                timestamp: 1623456789000, // Fixed timestamp in milliseconds
                block_height: height,
                block_hash: hex::encode(&hash),
                transaction_id: None,
            },
            payload: CdcPayload {
                operation: CdcOperation::Delete,
                table: "blocks".to_string(),
                key: height.to_string(),
                before: Some(serde_json::json!({
                    "height": height,
                    "hash": hex::encode(&hash),
                    "timestamp": 1623456789 // Fixed timestamp in seconds
                })),
                after: None,
            },
        };
        
        debshrew_runtime::println!("BlockTransform: Created rollback CDC message");
        
        // Return the CDC messages
        debshrew_runtime::println!("BlockTransform: Rollback complete");
        
        Ok(vec![message])
    }
}

// Register the transform
debshrew_runtime::declare_transform!(BlockTransform);
