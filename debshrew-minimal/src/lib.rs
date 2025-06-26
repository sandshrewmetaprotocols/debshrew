//! Minimal debshrew transform for testing
//!
//! This transform provides a simple implementation that can be used for
//! end-to-end testing of the debshrew framework. It mimics the behavior
//! of metashrew-minimal but generates CDC messages instead of just tracking blocks.

use debshrew_runtime::{
    declare_transform, view, get_height, CdcMessage, CdcHeader, CdcOperation, CdcPayload,
    serde_json, write_stdout, write_stderr, DebTransform,
};
use debshrew_runtime::Result as DebResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Minimal transform state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MinimalTransform {
    /// Track processed blocks by height -> first byte of block hash
    pub block_tracker: HashMap<u32, u8>,
    
    /// Track the last processed height
    pub last_height: Option<u32>,
}

impl debshrew_runtime::transform::DebTransform for MinimalTransform {
    fn process_block(&mut self) -> DebResult<Vec<CdcMessage>> {
        let height = get_height();
        
        write_stdout(&format!("Processing block at height {}", height));
        
        // Call the blocktracker view to get the current state
        let blocktracker_data = match view("blocktracker".to_string(), vec![]) {
            Ok(data) => data,
            Err(e) => {
                write_stderr(&format!("Failed to call blocktracker view: {}", e));
                return Err(e);
            }
        };
        
        write_stdout(&format!("Blocktracker data length: {}", blocktracker_data.len()));
        
        // The blocktracker should have (height + 1) bytes
        let expected_length = (height + 1) as usize;
        if blocktracker_data.len() != expected_length {
            let error_msg = format!(
                "Unexpected blocktracker length: got {}, expected {}",
                blocktracker_data.len(),
                expected_length
            );
            write_stderr(&error_msg);
            return Err(anyhow::anyhow!(error_msg));
        }
        
        // Generate CDC messages for the new block
        let mut cdc_messages = Vec::new();
        
        // If this is not the first block, check what changed
        if let Some(last_height) = self.last_height {
            if height != last_height + 1 {
                write_stderr(&format!(
                    "Non-sequential block: last={}, current={}",
                    last_height, height
                ));
            }
        }
        
        // Get the first byte of the current block hash from blocktracker
        if let Some(&block_hash_byte) = blocktracker_data.get(height as usize) {
            // Create a CDC message for the new block
            let cdc_message = CdcMessage {
                header: CdcHeader {
                    source: "debshrew-minimal".to_string(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                    block_height: height,
                    block_hash: format!("{:02x}", block_hash_byte),
                    transaction_id: None,
                },
                payload: CdcPayload {
                    operation: CdcOperation::Create,
                    table: "blocks".to_string(),
                    key: height.to_string(),
                    before: None,
                    after: Some(serde_json::json!({
                        "height": height,
                        "hash_byte": block_hash_byte,
                        "tracker_length": blocktracker_data.len()
                    })),
                },
            };
            
            cdc_messages.push(cdc_message);
            
            // Update our internal state
            self.block_tracker.insert(height, block_hash_byte);
        } else {
            let error_msg = format!("Block hash byte not found at index {}", height);
            write_stderr(&error_msg);
            return Err(anyhow::anyhow!(error_msg));
        }
        
        // Update last processed height
        self.last_height = Some(height);
        
        write_stdout(&format!("Generated {} CDC messages", cdc_messages.len()));
        
        Ok(cdc_messages)
    }
    
    fn rollback(&mut self) -> DebResult<Vec<CdcMessage>> {
        let height = get_height();
        
        write_stdout(&format!("Rolling back to height {}", height));
        
        let mut cdc_messages = Vec::new();
        
        // Generate inverse CDC messages for blocks that need to be rolled back
        if let Some(last_height) = self.last_height {
            for rollback_height in (height + 1)..=last_height {
                if let Some(&block_hash_byte) = self.block_tracker.get(&rollback_height) {
                    let cdc_message = CdcMessage {
                        header: CdcHeader {
                            source: "debshrew-minimal".to_string(),
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64,
                            block_height: height, // Target height for rollback
                            block_hash: format!("{:02x}", block_hash_byte),
                            transaction_id: None,
                        },
                        payload: CdcPayload {
                            operation: CdcOperation::Delete,
                            table: "blocks".to_string(),
                            key: rollback_height.to_string(),
                            before: Some(serde_json::json!({
                                "height": rollback_height,
                                "hash_byte": block_hash_byte,
                                "tracker_length": rollback_height + 1
                            })),
                            after: None,
                        },
                    };
                    
                    cdc_messages.push(cdc_message);
                    
                    // Remove from our internal state
                    self.block_tracker.remove(&rollback_height);
                }
            }
        }
        
        // Update last processed height
        self.last_height = Some(height);
        
        write_stdout(&format!("Generated {} rollback CDC messages", cdc_messages.len()));
        
        Ok(cdc_messages)
    }
}

// Declare the transform using the macro
declare_transform!(MinimalTransform);

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_minimal_transform_creation() {
        let transform = MinimalTransform::default();
        assert!(transform.block_tracker.is_empty());
        assert!(transform.last_height.is_none());
    }
    
    #[test]
    fn test_minimal_transform_state() {
        let mut transform = MinimalTransform::default();
        transform.block_tracker.insert(0, 0xaa);
        transform.block_tracker.insert(1, 0xbb);
        transform.last_height = Some(1);
        
        assert_eq!(transform.block_tracker.len(), 2);
        assert_eq!(transform.last_height, Some(1));
        assert_eq!(transform.block_tracker.get(&0), Some(&0xaa));
        assert_eq!(transform.block_tracker.get(&1), Some(&0xbb));
    }
}