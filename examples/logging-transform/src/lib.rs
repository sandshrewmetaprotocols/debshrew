//! A simple transform module that demonstrates logging with println!
//!
//! This transform module logs information about the block being processed
//! and generates CDC messages for each block.

use debshrew_runtime::{
    declare_transform, DebTransform,
    get_height, get_block_hash,
};
use debshrew_runtime::println;
use debshrew_support::{CdcMessage, CdcHeader, CdcOperation, CdcPayload};
use std::fmt::Debug;
use serde::{Serialize, Deserialize};

/// A simple transform that logs information about blocks
#[derive(Default, Clone, Serialize, Deserialize, Debug)]
pub struct LoggingTransform {
    /// The number of blocks processed
    blocks_processed: u32,
}

impl DebTransform for LoggingTransform {
    fn process_block(&mut self) -> debshrew_runtime::error::Result<Vec<CdcMessage>> {
        // Get block information
        let height = get_height();
        let block_hash = get_block_hash();
        
        // Increment the counter
        self.blocks_processed += 1;
        
        // Log information about the block
        println!("Processing block #{} at height {}", self.blocks_processed, height);
        println!("Block hash: {:?}", block_hash);
        
        // Get current timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        // Create a CDC message
        let message = CdcMessage {
            header: CdcHeader {
                source: "logging-transform".to_string(),
                timestamp,
                block_height: height,
                block_hash: hex::encode(&block_hash),
                transaction_id: Some(format!("tx-{}", self.blocks_processed)),
            },
            payload: CdcPayload {
                operation: CdcOperation::Create,
                table: "blocks".to_string(),
                key: format!("block-{}", height),
                before: None,
                after: Some(serde_json::json!({
                    "height": height,
                    "block_hash": hex::encode(&block_hash),
                    "blocks_processed": self.blocks_processed,
                    "timestamp": timestamp,
                })),
            },
        };
        
        println!("Generated CDC message: {}", serde_json::to_string_pretty(&message).unwrap());
        
        Ok(vec![message])
    }

    fn rollback(&mut self) -> debshrew_runtime::error::Result<Vec<CdcMessage>> {
        // Get block information
        let height = get_height();
        
        // Decrement the counter
        if self.blocks_processed > 0 {
            self.blocks_processed -= 1;
        }
        
        // Log information about the rollback
        println!("Rolling back block at height {}", height);
        println!("Blocks processed after rollback: {}", self.blocks_processed);
        
        // Get current timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        // Create a CDC message for the rollback
        let message = CdcMessage {
            header: CdcHeader {
                source: "logging-transform".to_string(),
                timestamp,
                block_height: height,
                block_hash: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                transaction_id: Some(format!("tx-{}", self.blocks_processed)),
            },
            payload: CdcPayload {
                operation: CdcOperation::Delete,
                table: "blocks".to_string(),
                key: format!("block-{}", height),
                before: Some(serde_json::json!({
                    "height": height,
                    "blocks_processed": self.blocks_processed + 1,
                    "timestamp": timestamp,
                })),
                after: None,
            },
        };
        
        println!("Generated CDC message for rollback: {}", serde_json::to_string_pretty(&message).unwrap());
        
        Ok(vec![message])
    }
}

// Declare the transform module
declare_transform!(LoggingTransform);