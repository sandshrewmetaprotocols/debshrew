//! Test utilities for debshrew-runtime
//!
//! This module provides utilities for testing WASM modules in the debshrew runtime.

use crate::imports::exports::{
    clear_test_state, set_test_height, set_test_hash
};
use crate::transform::DebTransform;
use crate::error::Result;
use debshrew_support::CdcMessage;
use std::path::Path;
use std::fs;

/// A test runner for WASM modules
pub struct TestRunner {
    /// The current block height
    pub height: u32,
    /// The current block hash
    pub hash: Vec<u8>,
}

impl Default for TestRunner {
    fn default() -> Self {
        Self {
            height: 0,
            hash: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        }
    }
}

impl TestRunner {
    /// Create a new test runner
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the block height for testing
    pub fn with_height(mut self, height: u32) -> Self {
        self.height = height;
        self
    }

    /// Set the block hash for testing
    pub fn with_hash(mut self, hash: Vec<u8>) -> Self {
        self.hash = hash;
        self
    }

    /// Initialize the test environment
    pub fn init(&self) {
        clear_test_state();
        set_test_height(self.height);
        set_test_hash(self.hash.clone());
    }

    /// Run a transform module
    pub fn run_transform<T: DebTransform>(&self, transform: &mut T) -> Result<Vec<CdcMessage>> {
        self.init();
        transform.process_block()
    }

    /// Run a rollback on a transform module
    pub fn run_rollback<T: DebTransform>(&self, transform: &mut T) -> Result<Vec<CdcMessage>> {
        self.init();
        transform.rollback()
    }

    /// Load a WASM module from a file
    pub fn load_wasm_from_file<P: AsRef<Path>>(path: P) -> Result<Vec<u8>> {
        let wasm_bytes = fs::read(path)
            .map_err(|e| anyhow::anyhow!("Failed to read WASM file: {}", e))?;
        Ok(wasm_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transform::DebTransform;
    use debshrew_support::{CdcMessage, CdcHeader, CdcOperation, CdcPayload};
    use serde::{Serialize, Deserialize};

    #[derive(Debug, Default, Clone, Serialize, Deserialize)]
    struct TestTransform {
        counter: u32,
    }

    impl DebTransform for TestTransform {
        fn process_block(&mut self) -> Result<Vec<CdcMessage>> {
            self.counter += 1;
            
            let message = CdcMessage {
                header: CdcHeader {
                    source: "test".to_string(),
                    timestamp: 0,
                    block_height: 123,
                    block_hash: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                    transaction_id: None,
                },
                payload: CdcPayload {
                    operation: CdcOperation::Create,
                    table: "test_table".to_string(),
                    key: "test_key".to_string(),
                    before: None,
                    after: Some(serde_json::json!({
                        "counter": self.counter
                    })),
                },
            };
            
            Ok(vec![message])
        }

        fn rollback(&mut self) -> Result<Vec<CdcMessage>> {
            self.counter -= 1;
            
            let message = CdcMessage {
                header: CdcHeader {
                    source: "test".to_string(),
                    timestamp: 0,
                    block_height: 123,
                    block_hash: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                    transaction_id: None,
                },
                payload: CdcPayload {
                    operation: CdcOperation::Delete,
                    table: "test_table".to_string(),
                    key: "test_key".to_string(),
                    before: Some(serde_json::json!({
                        "counter": self.counter + 1
                    })),
                    after: None,
                },
            };
            
            Ok(vec![message])
        }
    }

    #[test]
    #[cfg(feature = "test-utils")]
    fn test_runner() {
        let runner = TestRunner::new()
            .with_height(123)
            .with_hash(vec![1, 2, 3, 4]);
        
        let mut transform = TestTransform::default();
        
        // Run process_block
        let messages = runner.run_transform(&mut transform).unwrap();
        assert_eq!(messages.len(), 1);
        
        if let Some(json) = &messages[0].payload.after {
            assert_eq!(json["counter"], 1);
        } else {
            panic!("Expected JSON payload");
        }
        
        // Run rollback
        let messages = runner.run_rollback(&mut transform).unwrap();
        assert_eq!(messages.len(), 1);
        
        if let Some(json) = &messages[0].payload.before {
            assert_eq!(json["counter"], 1);
        } else {
            panic!("Expected JSON payload");
        }
    }
}